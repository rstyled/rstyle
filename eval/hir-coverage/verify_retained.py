"""Independently check retained byte identities, manifests and pinned schemas."""
import gzip
import hashlib
import json
from pathlib import Path
import jsonschema


def main():
    root = Path(__file__).resolve().parent
    vendor = root.parents[1]/'vendor-compat/rust-r0'
    validators = {kind: jsonschema.Draft202012Validator(json.loads((vendor/f'{kind}.schema.json').read_text())) for kind in ['pdg', 'cfg']}
    records = json.loads((root/'retained-index.json').read_text())
    checked = 0
    for capture in records:
        artifacts = {a['path']: a for a in capture['artifacts']}
        for path, artifact in artifacts.items():
            compressed = (root/path).read_bytes()
            assert hashlib.sha256(compressed).hexdigest() == artifact['compressed_sha256'], path
            data = gzip.decompress(compressed)
            assert len(data) == artifact['original_bytes'], path
            assert hashlib.sha256(data).hexdigest() == artifact['original_sha256'], path
            if path.endswith('.manifest.json.gz'):
                manifest = json.loads(data)
                for document in manifest['documents']:
                    kind = document['key'].rsplit(':', 1)[1]
                    target = path.removesuffix('.manifest.json.gz')+f'.{kind}.json.gz'
                    assert document['sha256'] == 'sha256:'+artifacts[target]['original_sha256'], target
                    value = json.loads(gzip.decompress((root/target).read_bytes()))
                    validators[kind].validate(value)
                    checked += 1
    print(f'Checked exact artifact hashes and manifests; {checked} serialized documents satisfy unchanged pinned schemas')


if __name__ == '__main__':
    main()
