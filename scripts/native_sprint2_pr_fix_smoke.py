#!/usr/bin/env python3
"""Real GPUI PR Fix review using an owned, offline GitHub CLI fixture.

The app owns all task, review and draft mutations. SQLite is read-only evidence.
The fixture implements read-only REST/GraphQL responses and rejects PR writes.
No production credential, remote Git operation or provider account is used.
"""
import argparse
import json
import os
import subprocess
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import preference, close
from native_integrations_smoke import click, fill, fresh_probe
from native_project_import_smoke import task_events

CLI = r'''#!/usr/bin/python3
import json, sys, time
from pathlib import Path
root = Path(__file__).resolve().parent.parent
config = json.loads((root / 'provider.json').read_text())
args = sys.argv[1:]
assert args[0] == 'api' and args[args.index('--hostname')+1] == 'github.com'
method = args[args.index('--method')+1]
endpoint = args[-1]
body = json.load(sys.stdin) if '--input' in args else None
with (root / 'requests.jsonl').open('a') as out:
    out.write(json.dumps({'method': method, 'endpoint': endpoint, 'body': body}) + '\n')
sha = config['sha']
pr = {'number': 7, 'title': 'Review fixture', 'state': 'open', 'draft': False,
      'user': {'login':'reviewer'}, 'head':{'sha':sha,'ref':'review'},
      'base':{'sha':sha,'ref':'main'}, 'merged':False, 'node_id':'PR_fixture',
      'html_url':'https://github.com/owner/project/pull/7'}
if method == 'POST' and endpoint == 'graphql':
    assert body['query'].startswith('query ') and 'mutation' not in body['query']
    v = body['variables']
    assert v['owner'] == 'owner' and v['name'] == 'project' and v['number'] == 7
    time.sleep(config.get('delay', 0))
    if v['after'] is None:
        nodes = [{'id':'resolved-'+str(i),'isResolved':True} for i in range(25)]
        more, cursor = True, 'second-page'
    else:
        assert v['after'] == 'second-page'
        comment = {'id':'PRRC_fixture', 'url':'https://github.com/owner/project/pull/7#discussion_r12',
          'body':config.get('body','Check this test fixture carefully.'), 'path':'document.txt',
          'diffHunk':'@@ -1 +1 @@\n-original text\n+reviewed text',
          'updatedAt':'2026-09-22T00:00:00Z', 'originalCommit':{'oid':sha},
          'commit':{'oid':sha}, 'author':{'login':'reviewer'}}
        nodes = [{'id':'PRRT_fixture','isResolved':config.get('resolved',False),
          'isOutdated':False,'path':'document.txt','diffSide':'RIGHT','startDiffSide':None,
          'subjectType':'LINE','line':1,'startLine':None,'originalLine':1,'originalStartLine':None,
          'comments':{'totalCount':1,'pageInfo':{'hasNextPage':False},'nodes':[comment]}}]
        more, cursor = False, None
    result = {'data':{'repository':{'nameWithOwner':'owner/project','pullRequest':{
      'number':7,'state':'OPEN','headRefOid':sha,'reviewThreads':{'totalCount':26,
      'pageInfo':{'hasNextPage':more,'endCursor':cursor},'nodes':nodes}}}}}
elif method == 'GET' and endpoint.startswith('search/issues?'):
    result = {'incomplete_results':False,'items':[pr]}
elif method == 'GET' and endpoint == 'repos/owner/project/pulls/7':
    result = pr
elif method == 'GET' and '/check-runs?' in endpoint:
    result = {'check_runs':[]}
elif method == 'GET' and '/status?' in endpoint:
    result = {'statuses':[]}
elif method == 'GET' and endpoint.startswith('repos/owner/project/') and any(
    part in endpoint for part in ['/files?', '/commits?', '/timeline?', '/reviews?']):
    result = []
else:
    raise AssertionError('Unexpected request or forbidden remote write: '+method+' '+endpoint)
print(json.dumps(result))
'''


def requests(s):
    path = s.output / 'requests.jsonl'
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def graph_count(s):
    return sum(row['endpoint'] == 'graphql' for row in requests(s))


def config(s, sha, **extra):
    path = s.output / 'provider.next'
    path.write_text(json.dumps({'sha':sha, **extra}))
    path.replace(s.output / 'provider.json')


def draft(s, task):
    return (preference(s, 'task-draft:' + task) or {}).get('text', '')


def review_text(s):
    click(s, 'pr-fix-editor')
    return s.desktop.copy_input()


def run(s):
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=s.project, text=True).strip()
    git('init', '-q', '--initial-branch=main')
    git('config', 'user.name', 'PR Fix native fixture')
    git('config', 'user.email', 'pr-fix@example.invalid')
    git('add', 'document.txt')
    git('commit', '-qm', 'Owned fixture baseline')
    git('remote', 'add', 'origin', 'https://github.com/owner/project.git')
    sha = git('rev-parse', 'HEAD')
    original_index = (s.project / '.git/index').read_bytes()
    original_document = s.document.read_bytes()
    bin_dir = s.output / 'bin'
    bin_dir.mkdir()
    gh = bin_dir / 'gh'
    gh.write_text(CLI)
    gh.chmod(0o700)
    os.environ['PATH'] = str(bin_dir) + os.pathsep + os.environ['PATH']
    config(s, sha)
    s.launch()
    task = selection(s)
    original = task_events(s, task)
    fill(s, 'composer-input', 'Keep my original unsent request')
    wait_until(lambda: draft(s, task) == 'Keep my original unsent request', 'initial draft persistence')
    click(s, 'pull-requests-navigation')
    click(s, 'pr-discover')
    click(s, 'pr-remote', slot=0)
    click(s, 'pr-row', slot=0)
    click(s, 'pr-collect-fixes')
    text = review_text(s)
    assert 'PRRT_fixture' in text and 'PRRC_fixture' in text
    assert 'discussion_r12' in text and sha in text and 'document.txt' in text
    assert 'resolved-0' not in text and graph_count(s) == 2
    assert draft(s, task) == 'Keep my original unsent request'
    assert task_events(s, task) == original and task_count(s) == 1
    s.checks.append('native-paginated-read-only-collection-preserves-review-identity-and-leaves-draft-inert')
    s.desktop.screenshot('pr-fix-collected', window_only=True)

    s.desktop.focus()
    s.desktop.key('End', ('Control_L',))
    s.desktop.key('Return')
    s.desktop.text('Use my focused check only')
    edited = review_text(s)
    assert edited.endswith('Use my focused check only') and 'PRRC_fixture' in edited
    config(s, sha, body='A reviewer changed this comment.')
    fresh_probe(s, 'pr-error', lambda: click(s, 'pr-fix-add'))
    assert review_text(s) == edited and draft(s, task) == 'Keep my original unsent request'
    assert task_events(s, task) == original
    s.checks.append('edited-review-is-kept-and-stale-comments-block-draft-insertion')

    config(s, 'b' * 40)
    fresh_probe(s, 'pr-error', lambda: click(s, 'pr-fix-add'))
    assert review_text(s) == edited and draft(s, task) == 'Keep my original unsent request'
    s.checks.append('changed-reviewed-head-is-rejected-without-checkout-or-draft-mutation')

    config(s, sha, delay=1.5)
    before = graph_count(s)
    click(s, 'pr-fix-add')
    wait_until(lambda: graph_count(s) > before, 'in-flight review verification')
    click(s, 'pr-cancel')
    time.sleep(1.8)
    assert review_text(s) == edited and draft(s, task) == 'Keep my original unsent request'
    assert task_events(s, task) == original
    s.checks.append('cancelled-verification-keeps-user-edits-and-late-completion-cannot-insert')

    config(s, sha)
    click(s, 'pr-fix-add')
    expected = 'Keep my original unsent request\n\n' + edited
    wait_until(lambda: draft(s, task) == expected, 'reviewed instructions appended once to original task draft')
    assert task_events(s, task) == original and task_count(s) == 1
    click(s, 'composer-input')
    assert s.desktop.copy_input() == expected
    s.checks.append('explicit-fresh-handoff-appends-the-user-edited-instructions-to-the-correct-unsent-draft')
    s.desktop.screenshot('pr-fix-unsent-draft', window_only=True)

    click(s, 'pull-requests-navigation')
    config(s, sha, resolved=True)
    fresh_probe(s, 'pr-error', lambda: click(s, 'pr-collect-fixes'))
    assert draft(s, task) == expected and task_events(s, task) == original
    s.checks.append('empty-unresolved-review-does-not-create-a-fix-prompt')

    config(s, sha)
    click(s, 'pr-collect-fixes')
    assert 'PRRC_fixture' in review_text(s)
    click(s, 'pr-fix-discard')
    assert 'PRRC_fixture' in review_text(s)
    click(s, 'pr-fix-keep')
    assert 'PRRC_fixture' in review_text(s)
    click(s, 'pr-fix-discard')
    click(s, 'pr-fix-discard-confirm')
    assert draft(s, task) == expected
    s.checks.append('discard-requires-confirmation-and-never-removes-the-existing-chat-draft')

    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == task and draft(s, task) == expected
    assert task_events(s, task) == original and task_count(s) == 1
    assert git('rev-parse', 'HEAD') == sha
    assert (s.project / '.git/index').read_bytes() == original_index
    assert s.document.read_bytes() == original_document
    assert all(row['method'] == 'GET' or (row['method'] == 'POST' and row['endpoint'] == 'graphql'
        and row['body']['query'].startswith('query ')) for row in requests(s))
    s.checks.append('restart-restores-only-the-normal-unsent-draft-with-no-provider-execution-or-git-write')


def main():
    parser = argparse.ArgumentParser()
    for name in ('binary', 'fixture', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = {'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb and offline gh fixture'}
    try:
        run(s)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if s.process and s.process.poll() is None:
            s.desktop.screenshot('failure')
        raise
    finally:
        s.close()
        (s.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
