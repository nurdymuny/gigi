"""Retain primary-source metadata and inspect archives without executing them.

This is data discovery, not a physics or classification benchmark.
"""
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import urllib.request
import zipfile


def fetch(url, path):
    data = urllib.request.urlopen(url, timeout=90).read()
    path.write_bytes(data)
    return data, dict(url=url, path=path.name, bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    report = dict(status='SOURCE_DISCOVERY_ONLY', sources=[])
    for domain, article in [('virus',14420588), ('aerospace',12707963)]:
        data, receipt = fetch(f'https://api.figshare.com/v2/articles/{article}', args.output/f'{domain}-metadata.json')
        metadata = json.loads(data)
        entry = dict(domain=domain, title=metadata['title'], version=metadata['version'],
                     license=metadata['license'], metadata=receipt, files=metadata['files'], downloaded=[])
        for file in metadata['files']:
            # Published figure workbooks are small; the aircraft flight archive is 272 MB.
            if domain == 'aerospace' and file['name'] not in ('README.txt','Version History.txt'):
                continue
            path = args.output/f"{domain}-{file['name']}"
            data, receipt = fetch(file['download_url'], path)
            assert len(data) == file['size'], 'download size disagrees with primary metadata'
            if file['name'].endswith('.xlsx'):
                with zipfile.ZipFile(io.BytesIO(data)) as archive:
                    assert archive.testzip() is None
                    receipt['xlsx_archive_integrity'] = True
            entry['downloaded'].append(receipt)
        report['sources'].append(entry)
    url='https://php.lobsterdata.com/info/sample/LOBSTER_SampleFile_AAPL_2012-06-21_1.zip'
    data, receipt = fetch(url,args.output/'AAPL-sample.zip')
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        assert archive.testzip() is None
        entry = dict(domain='finance', source=receipt, files=[])
        for name in archive.namelist():
            payload=archive.read(name)
            file=dict(name=name, bytes=len(payload), sha256=hashlib.sha256(payload).hexdigest())
            if name.endswith('.csv'):
                rows=list(csv.reader(io.StringIO(payload.decode('utf-8-sig'))))
                file.update(rows=len(rows), column_counts=sorted(set(map(len, rows))), first_row=rows[0], last_row=rows[-1])
            entry['files'].append(file)
        counts=[f['rows'] for f in entry['files'] if 'rows' in f]
        assert len(counts)==2 and counts[0]==counts[1], 'message/book alignment requires equal row counts'
        report['sources'].append(entry)
    (args.output/'inventory.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps({s['domain']:len(s.get('downloaded',s.get('files',[]))) for s in report['sources']}))


if __name__=='__main__':
    main()
