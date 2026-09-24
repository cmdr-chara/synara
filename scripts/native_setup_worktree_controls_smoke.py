#!/usr/bin/env python3
"""Native setup authentication, composer controls and reviewed isolated forks.

Only owned ACP fixtures, temporary repositories and private Xvfb are used.
"""
import argparse
import json
import subprocess
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_controls_smoke import option, config_count, settled_control_count, wait_applied_render
from native_model_draft_smoke import preference, close
from native_integrations_smoke import fill, click
from native_studio_settings_smoke import tasks


def choose(s, text):
    fill(s, 'choice-search', text)
    s.desktop.key('Return')
    time.sleep(0.25)


def setup(s):
    profiles = json.loads(s.profiles.read_text())[:1]
    profiles[0].update(id='auth', name='Fixture sign-in', args=['--integration-fixture', 'auth'])
    s.profiles.write_text(json.dumps(profiles))
    s.launch(preserve_selection=True)  # Real empty-install onboarding, no CLI project.
    assert task_count(s) == 0 and not s.events()
    click(s, 'onboarding-next')
    click(s, 'onboarding-next')
    click(s, 'onboarding-agent-prepare', slot=0)
    wait_until(lambda: task_count(s) == 1 and selection(s), 'saved setup chat')
    task = selection(s)
    assert tasks(s)[task]['scope'] == 'chat'
    assert not s.events(), 'preparation must not start an agent or a turn'
    click(s, 'onboarding-agent-connect', slot=0)
    wait_until(lambda: s.control_bounds('onboarding-auth-method', slot=0), 'agent-advertised authentication')
    assert not any(e['type'] == 'prompt_started' for e in s.events())
    click(s, 'onboarding-auth-method', slot=0)
    wait_until(lambda: option(s, 'model') == 'auth', 'authenticated fixture session')
    before = config_count(s)
    s.desktop.key('bracketright', ('Alt_L',))
    time.sleep(0.2)
    assert config_count(s) == before, 'Settings input must not cycle the composer model'
    s.desktop.screenshot('setup-authenticated-fixture', window_only=True)
    s.checks.append('fresh-setup-chat-is-inert-until-explicit-connect-and-advertised-login')
    s.desktop.key('1', ('Control_L',))
    fill(s, 'composer-input', 'Keep this unsent draft')
    before = settled_control_count(s)
    s.desktop.key('bracketright', ('Alt_L',))
    wait_applied_render(s, before)
    assert option(s, 'model') == 'alternate'
    s.click_control('composer-input')
    assert s.desktop.copy_input() == 'Keep this unsent draft'
    before = settled_control_count(s)
    s.desktop.key('bracketleft', ('Alt_L',))
    wait_applied_render(s, before)
    assert option(s, 'model') == 'auth'
    before = settled_control_count(s)
    fill(s, 'composer-input', '/synara/model next')
    s.click_control('composer-submit', enabled=True)
    wait_applied_render(s, before)
    assert option(s, 'model') == 'alternate'
    fill(s, 'composer-input', '/synara/model previous extra')
    s.click_control('composer-submit', enabled=True)
    s.click_control('composer-input')
    assert s.desktop.copy_input() == '/synara/model previous extra'
    assert option(s, 'model') == 'alternate'
    assert not any(e['type'] == 'prompt_started' for e in s.events())
    events = s.events()
    close(s)
    s.launch(preserve_selection=True)
    assert task_count(s) == 1 and selection(s) == task
    assert s.events() == events, 'restart must not replay setup authentication or commands'
    s.checks.append('composer-only-model-shortcuts-preserve-draft-and-qualified-commands-never-send')
    close(s)


def worktree(s):
    def git(*args):
        return subprocess.check_output(['git', '-C', str(s.project), *args], text=True).strip()
    git('init', '-q', '--initial-branch=main')
    git('config', 'user.name', 'Synara fixture')
    git('config', 'user.email', 'fixture@example.invalid')
    git('add', '.')
    git('-c', 'commit.gpgSign=false', 'commit', '-qm', 'initial fixture')
    base = git('rev-parse', 'HEAD')
    s.launch()
    start = s.prompt('hello')
    s.finished(start)
    original = selection(s)
    s.document.write_text('source dirty text\n')
    s.click_control('branch-message-worktree')
    choose(s, 'Create isolated worktree')
    time.sleep(0.3)  # The async review is a distinct, inert menu.
    assert task_count(s) == 1, 'review must not create a task'
    assert not list((s.data / 'chats').glob('worktree-*')), 'review must not checkout'
    s.desktop.screenshot('isolated-worktree-review', window_only=True)
    choose(s, 'Allow checkout')
    wait_until(lambda: task_count(s) == 2 and selection(s) != original, 'unsent isolated fork')
    fork = selection(s)
    target = Path(tasks(s)[fork]['working_directory'])
    assert target != s.project and target.name.startswith('worktree-')
    assert (target / 'document.txt').read_text() == 'original text\n'
    assert s.document.read_text() == 'source dirty text\n'
    assert git('rev-parse', 'HEAD') == base and git('branch', '--show-current') == 'main'
    assert subprocess.check_output(['git', '-C', str(target), 'rev-parse', 'HEAD'], text=True).strip() == base
    assert 'Source task: ' + original in preference(s, 'task-draft:' + fork)['text']
    assert sum(e['type'] == 'prompt_started' for e in s.events()) == 1
    events = s.events()
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == fork, 'restart must select the same isolated task'
    restored_directory = Path(tasks(s)[fork]['working_directory'])
    assert restored_directory == target, 'restart must retain the isolated working directory'
    assert target.is_dir() and s.events() == events
    s.checks.append('reviewed-pinned-worktree-fork-keeps-dirty-source-and-persists-without-autostart')
    close(s)


def main():
    parser = argparse.ArgumentParser()
    for name in ['binary', 'fixture', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    checks = []
    for name, run in [('setup', setup), ('worktree', worktree)]:
        s = Scenario(argparse.Namespace(binary=args.binary, fixture=args.fixture, output=args.output / name))
        result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb'}
        try:
            run(s)
            result['status'] = 'passed'
            checks.extend(s.checks)
        except BaseException as error:
            result['error'] = str(error)
            if s.process and s.process.poll() is None:
                s.desktop.screenshot('failure')
            raise
        finally:
            s.close()
            (s.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    (args.output / 'result.json').write_text(json.dumps({'status': 'passed', 'checks': checks}, indent=2) + '\n')


if __name__ == '__main__':
    main()
