#!/usr/bin/env python3
"""Expose nextest failure details in CI annotations, including public API output."""
from pathlib import Path
import sys
import xml.etree.ElementTree as ET


def escape(value):
    return (value.replace('%', '%25').replace('\r', '%0D')
            .replace('\n', '%0A'))


def annotate(path):
    if not path.is_file():
        print('No nextest JUnit report was produced; inspect the failed build step.')
        return
    root = ET.parse(path).getroot()
    for case in root.iter('testcase'):
        for failure in list(case.findall('failure')) + list(case.findall('error')):
            title = case.get('classname', '') + '::' + case.get('name', '')
            output = [failure.get('message', ''), failure.text or '']
            output.extend(case.findtext(tag, '') for tag in ['system-out', 'system-err'])
            title = escape(title).replace(':', '%3A').replace(',', '%2C')
            print('::error title=' + title + '::' + escape('\n'.join(output)[:12000]))


if __name__ == '__main__':
    annotate(Path(sys.argv[1]))
