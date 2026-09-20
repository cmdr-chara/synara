#!/usr/bin/env python3
"""The Kanban probe must recognize streaming without joining unrelated messages."""
import unittest
from native_kanban_smoke import assistant_has_text

class StreamChecks(unittest.TestCase):
    def test_one_answer_can_arrive_in_several_chunks(self):
        events = [dict(type='text_delta', role='assistant', message_id='answer', text=text)
                  for text in ['Hello ', 'from ', 'beta']]
        self.assertTrue(assistant_has_text(events, 'Hello from beta'))
    def test_other_messages_and_user_text_do_not_prove_the_answer(self):
        events = [dict(type='text_delta', role='assistant', message_id='first', text='Hello '),
                  dict(type='text_delta', role='assistant', message_id='second', text='from beta'),
                  dict(type='text_delta', role='user', message_id='user', text='Hello from beta')]
        self.assertFalse(assistant_has_text(events, 'Hello from beta'))

if __name__ == '__main__':
    unittest.main()
