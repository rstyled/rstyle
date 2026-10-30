"""Read a completed compiler workspace evaluation for research scorers.

Recorded flat reports remain readable. This adapter only selects published rows;
it does not mine proposals or replace missing inputs with an empty result.
"""

def evaluation(report):
    if 'compiler_advisory' not in report:
        if not all(key in report for key in ('candidates', 'signature_groups', 'suppressed')):
            raise ValueError('missing recorded pattern evaluation')
        return report
    if report.get('capture_complete') is not True or report.get('assessment_complete') is not True:
        raise ValueError('compiler pattern capture is incomplete')
    populations = [unit['patterns'] for unit in report['compiler_advisory']['units']
                   if 'patterns' in unit]
    if not populations or any(population.get('input_complete') is not True for population in populations):
        raise ValueError('compiler pattern inputs are missing or incomplete')
    first = populations[0]['evaluation']
    if any(population['evaluation'] != first for population in populations[1:]):
        raise ValueError('expected one shared workspace pattern evaluation')
    return first
