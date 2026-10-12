"""Independent expected outcomes for evaluator bookkeeping, never quality evidence."""
import copy
import itertools
import json
import pathlib
import tempfile
import unittest
from unittest.mock import patch
import ccgraph_study_evidence as evidence
import ccgraph_study as s

class Bookkeeping(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.study = pathlib.Path(self.tmp.name)
        self.scope = dict(id='scope', split='development', population='controlled', source_functions=2, selected_units=2)
        path = self.study / 'sample/src/lib.rs'; path.parent.mkdir(parents=True); path.write_text('fn a() {}fn b() {}')
        self.fs = [dict(id=name, scope='scope', package='sample', target='sample', path='sample/src/lib.rs', start=i*9, end=i*9+9,
                        sha256=s.digest(path.read_bytes()[i*9:i*9+9]), family='family', provenance_group='project') for i,name in enumerate(['a','b'])]
        self.manifest = dict(scopes=[self.scope], functions=self.fs, labels=[dict(scope='scope',first='a',second='b',label='positive',rationale='Exact source copies differing only in function identifier.',stratum='rename')])
        units=[]; functions=[]
        for test in (False,True):
            directory=self.study / str(test)
            units.append(dict(directory=str(directory),test=test,target=dict(manifest_dir=str(self.study/'sample'),name='sample',kind=['lib'])))
            for f in self.fs:
                functions.append(dict(key=f['id']+str(test),artifact_prefix=str(directory/'function'),outcome='admitted',reasons=[],facts=dict(
                    fn_span=dict(start=f['start'],end=f['end']),source=dict(bytes=path.read_bytes()[f['start']:f['end']].decode(),digest='sha256:'+f['sha256']),
                    build=dict(frontend_options=dict(source_file=str(path))))))
        pairs=[dict(first=a['key'],second=b['key'],evaluation_status='evaluated',decision='Accepted',
                    stages=[dict(stage=stage,outcome='Passed',detail={}) for stage in ['Validation','Projection','Nonempty','SizeRatio','Numerical','Strings','WlFinal']]) for a,b in itertools.combinations(functions,2)]
        self.report=dict(observed_units=units,intended_primary_units=[{},{}],functions=functions,capture_failures=[],handoff_failures=[],contract_revision=s.CONTRACT,
                         matching=dict(incomplete=False,diagnostics=[],profile_id='compat.ccgraph.strict',profile_version='0.1.0',scoring='smaller_graph_overlap',
                                       effective_config=dict(s.DEFAULTS,scoring='smaller_graph_overlap'),functions=[dict(key=f['key']) for f in functions],pairs=pairs),**dict.fromkeys(s.FLAGS,True))

    def assess(self): return s.assess(self.report,self.scope,self.manifest,'default',self.study)

    def test_manifest_and_labels(self):
        s.validate_manifest(self.manifest,self.study)
        for mutate in [lambda m:m['labels'].clear(),lambda m:m['labels'].append(copy.deepcopy(m['labels'][0])),
                       lambda m:m['labels'][0].update(label='should refactor'),lambda m:m['labels'][0].update(rationale=''),
                       lambda m:m['functions'].append(copy.deepcopy(m['functions'][0])),lambda m:m['functions'][0].update(sha256='bad')]:
            m=copy.deepcopy(self.manifest); mutate(m)
            with self.assertRaises(s.Invalid): s.validate_manifest(m,self.study)

    def test_split_leakage_family_project_and_context(self):
        for group in ['family','provenance_group']:
            m=copy.deepcopy(self.manifest)
            m['scopes'].append(dict(self.scope,id='held',split='heldout',source_functions=1))
            f=dict(self.fs[0],id='third',scope='held',path='other.rs',family='other',provenance_group='other')
            f[group]=self.fs[0][group]; m['functions'].append(f)
            with self.assertRaisesRegex(s.Invalid,'leakage'): s.validate_manifest(m)

    def test_mapping_missing_ambiguous_and_duplicate(self):
        for mutate in [lambda r:r['functions'].pop(),lambda r:r['functions'].append(copy.deepcopy(r['functions'][0])),
                       lambda r:r['functions'][0]['facts']['fn_span'].update(start=1),lambda r:r['observed_units'].pop()]:
            r=copy.deepcopy(self.report); mutate(r)
            with self.assertRaises(s.Invalid): s.assess(r,self.scope,self.manifest,'default',self.study)
        self.manifest['functions'].append(copy.deepcopy(self.fs[0]))
        with self.assertRaisesRegex(s.Invalid,'ambiguous'): self.assess()

    def test_generated_entrypoint_retained_only_when_excluded(self):
        r=self.report
        unit=r['observed_units'][1]; unit['target']['source']=str(self.study/'sample/src/lib.rs'); unit['target']['package']='sample-package'
        generated=copy.deepcopy(r['functions'][0]); generated.update(key='generated',artifact_prefix=str(pathlib.Path(unit['directory'])/'entry'),outcome='excluded',reasons=['Macro-generated functions are deferred to rust.macros.'])
        generated['facts'].update(fn_span={'start':0,'end':0},source={'bytes':'','digest':'sha256:'+s.digest(b'')},source_scan_complete=False)
        r['functions'].append(generated)
        result=self.assess()
        self.assertEqual(result['functions']['selected_source_observed'],4)
        self.assertEqual(result['functions']['compiler_generated_observed'],1)
        self.assertEqual(result['declared_context_pair_universe'],6)
        generated['outcome']='admitted'
        with self.assertRaises(s.Invalid): self.assess()

    def test_pair_duplicate_missing_reverse_self_out_of_scope(self):
        for mutate in [lambda p:p.pop(),lambda p:p.append(copy.deepcopy(p[0])),lambda p:p.append(dict(p[0],first=p[0]['second'],second=p[0]['first'])),
                       lambda p:p[0].update(second=p[0]['first']),lambda p:p[0].update(second='alien')]:
            r=copy.deepcopy(self.report); mutate(r['matching']['pairs'])
            with self.assertRaises(s.Invalid): s.assess(r,self.scope,self.manifest,'default',self.study)

    def test_independent_confusion_and_denominators(self):
        def row(label,decision,status='evaluated',admitted=True):
            return dict(label=label,decision=decision,status=status,both_admitted=admitted,groups=['g'])
        rows=[row('positive','Accepted'),row('positive','Rejected'),row('positive',None,'excluded',False),
              row('negative','Accepted'),row('negative','Rejected'),row('negative','Rejected'),row('ambiguous','Accepted')]
        m=s.metrics(rows)
        self.assertEqual([m[k] for k in ('TP','FP','TN','FN')],[1,1,2,1])
        self.assertEqual(m['precision'],0.5); self.assertEqual(m['recall'],0.5)
        self.assertAlmostEqual(m['false_positive_rate'],1/3); self.assertEqual(m['f1'],0.5)
        self.assertAlmostEqual(m['positive_admission_coverage'],2/3)
        self.assertAlmostEqual(m['end_to_end_positive_yield'],1/3)
        self.assertEqual(m['unlabeled_accepted'],1)

    def test_undefined_and_insufficient(self):
        m=s.metrics([])
        for key in ['precision','recall','false_positive_rate','f1','end_to_end_positive_yield']: self.assertIsNone(m[key])
        self.assertTrue(all(t['outcome']=='insufficient evidence' for t in s.targets(m).values()))
        m.update(evaluable_positive=15,evaluable_negative=30,independent_groups=['a','b','c'],precision=.9,recall=.8,false_positive_rate=.1,positive_admission_coverage=.8,end_to_end_positive_yield=.64)
        self.assertTrue(all(t['outcome']=='met' for t in s.targets(m).values()))
        m['recall']=.79; self.assertEqual(s.targets(m)['recall']['outcome'],'not met')

    def test_exclusion_not_gate_rejection(self):
        self.report['functions'][0]['outcome']='excluded'; self.report['functions'][0]['reasons']=['unsupported call']
        key=self.report['functions'][0]['key']
        self.report['matching']['functions']=[f for f in self.report['matching']['functions'] if f['key']!=key]
        self.report['matching']['pairs']=[p for p in self.report['matching']['pairs'] if key not in (p['first'],p['second'])]
        m=self.assess()['metrics']; self.assertEqual(m['FN'],0); self.assertEqual(m['non_evaluable'],{'excluded':1}); self.assertEqual(m['end_to_end_positive_yield'],0)

    def test_incompatible_vs_failure(self):
        p=self.report['matching']['pairs'][0]; p.update(evaluation_status='incompatible',decision=None)
        p['stages'][0]['outcome']={'Ineligible':{'reason':'native width'}}
        m=self.assess()['metrics']; self.assertEqual(m['FN'],0); self.assertEqual(m['non_evaluable'],{'incompatible':1}); self.assertEqual(m['positive_admission_coverage'],1)
        p['evaluation_status']='failed'; p['stages'][0]['outcome']={'Error':{'reason':'overflow'}}
        with self.assertRaises(s.Invalid): self.assess()

    def test_incomplete_resource_arithmetic_and_malformed(self):
        for mutate in [lambda r:r.update(capture_complete=False),lambda r:r.update(pair_enumeration_complete=False),
                       lambda r:r['matching'].update(incomplete=True,diagnostics=['resource budget']),
                       lambda r:r['matching']['pairs'][0]['stages'].pop(),lambda r:r['functions'][0].update(outcome='failed'),
                       lambda r:r['matching']['pairs'][0]['stages'][0].update(outcome={'Error':{'reason':'arithmetic'}})]:
            r=copy.deepcopy(self.report); mutate(r)
            with self.assertRaises(s.Invalid): s.assess(r,self.scope,self.manifest,'default',self.study)

    def test_deterministic_order(self):
        expected=self.assess()
        self.report['functions'].reverse(); self.report['observed_units'].reverse(); self.report['matching']['functions'].reverse(); self.report['matching']['pairs'].reverse()
        self.assertEqual(self.assess(),expected)

    def test_saved_result_resource_and_aggregate_checks(self):
        result=self.assess()
        result['cost']={'exit_code':0,'wall_seconds':1,'fresh_run_bytes':1,'sampled_process_group_peak_rss_bytes':1}
        s.validate_result(result,self.scope,self.manifest,'default')
        for mutate in [lambda r:r['cost'].update(wall_seconds=181), lambda r:r['cost'].update(fresh_run_bytes=2*1024**3),
                       lambda r:r['metrics'].update(TP=10), lambda r:r['canonical_pairs'][0].update(decision='Rejected'),
                       lambda r:r['cost'].update(sampled_process_group_peak_rss_bytes=3*1024**3),lambda r:r['cost'].update(exit_code=2)]:
            r=copy.deepcopy(result);mutate(r)
            with self.assertRaises(s.Invalid):s.validate_result(r,self.scope,self.manifest,'default')

    def test_evidence_packaging_and_raw_drift(self):
        manifest_path=self.study/'manifest.json'; s.write(manifest_path,self.manifest)
        for variant in ('default','cosine'):
            result=self.assess();result['variant']=variant
            rawdir=self.study/('raw-'+variant);rawdir.mkdir()
            raw=copy.deepcopy(self.report)
            raw.update(run_directory=str(rawdir/'run'),toolchain='pinned-test-compiler',compiler_identity='compiler',cargo_identity='cargo',extraction_policy={},schema_versions={},scope={})
            for f in raw['functions']:f['facts']['build']['producer_revision']={'value':'fixture-revision'}
            s.write(rawdir/'stdout.json',raw);s.write(rawdir/'evidence-inventory.json',{'facts':'digest'})
            result.update(raw_directory=rawdir.name,raw_report_sha256=s.digest((rawdir/'stdout.json').read_bytes()),evidence_inventory_identity=s.identity({'facts':'digest'}),
                          cost={'exit_code':0,'wall_seconds':1,'fresh_run_bytes':1,'sampled_process_group_peak_rss_bytes':1})
            s.write(self.study/'results'/('scope-'+variant+'.json'),result)
        with patch.object(s,'ROOT',self.study),patch.object(s,'STUDY',self.study),patch.object(s,'validate_freeze',return_value={}):
            evidence.evidence()
            index=s.read(self.study/'results/evidence-index.json')
            self.assertEqual(index['scope-default']['producer_build_revisions'],['fixture-revision'])
            self.assertEqual(index['scope-default']['inventory'],{'facts':'digest'})
            (self.study/'raw-default/stdout.json').write_text('{}')
            with self.assertRaisesRegex(s.Invalid,'raw report drift'):evidence.evidence()

    def test_configuration_drift(self):
        self.report['matching']['effective_config']['final_threshold']='1/2'
        with self.assertRaisesRegex(s.Invalid,'config drift'): self.assess()

    def test_frozen_input_drift(self):
        root=self.study; path=root/'input'; path.write_text('frozen')
        freeze=dict(version=1,files={'input':s.digest(path.read_bytes())}); freeze['identity']=s.identity(freeze)
        s.write(root/'freeze.json',freeze); s.write(root/'manifest.json',dict(scopes=[],functions=[],labels=[]))
        s.validate_freeze(root,root)
        path.write_text('changed')
        with self.assertRaisesRegex(s.Invalid,'drift'): s.validate_freeze(root,root)

if __name__=='__main__': unittest.main()
