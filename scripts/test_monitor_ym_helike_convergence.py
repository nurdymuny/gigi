"""Logging contract: observe evidence without changing or duplicating it."""
import copy
import csv
import io
import json
import unittest

import monitor_ym_helike_convergence as monitor


class RecorderTests(unittest.TestCase):
    def fixture(self):
        return {'run_id':'fixture', 'source_revision':'revision', 'results':{'track':{'history':[
            {'g2':1., 'alpha':1.7, 'cutoff':8, 'E0':5., 'E1':7., 'absolute_gap':2.,
             'H0':{'lowest_weight':.4, 'accessible_gap':2.}, 'H2':{'lowest_weight':0., 'accessible_gap':3.},
             'exterior':{'level_worst_residuals':[.1, .2], 'allowed_residual':.02,
                         'residual_gate':False, 'tail_gate':True, 'probe_discarded_fractions':{'H0':.0001,'H2':.0002}}}
        ]}}}

    def test_records_exact_results_once_without_changing_receipt(self):
        receipt = self.fixture(); original = copy.deepcopy(receipt)
        journal, table, seen = io.StringIO(), io.StringIO(), set()
        writer = csv.DictWriter(table, fieldnames=monitor.COLUMNS); writer.writeheader()
        self.assertEqual(monitor.record_cases(receipt, seen, journal, writer, 'first', True), 1)
        self.assertEqual(monitor.record_cases(receipt, seen, journal, writer, 'second', False), 0)
        self.assertEqual(receipt, original)
        event = json.loads(journal.getvalue())
        self.assertTrue(event['already_saved_when_monitor_started'])
        self.assertEqual(event['case'], original['results']['track']['history'][0])
        rows = list(csv.DictReader(io.StringIO(table.getvalue())))
        self.assertEqual(len(rows), 1)
        self.assertEqual(float(rows[0]['max_residual']), .2)
        self.assertEqual(rows[0]['residual_gate'], 'False')

    def test_new_case_has_observed_not_invented_completion_time(self):
        journal, table, seen = io.StringIO(), io.StringIO(), set()
        writer = csv.DictWriter(table, fieldnames=monitor.COLUMNS); writer.writeheader()
        monitor.record_cases(self.fixture(), seen, journal, writer, 'observed-now', False)
        event = json.loads(journal.getvalue())
        self.assertFalse(event['already_saved_when_monitor_started'])
        self.assertEqual(event['observed_at_utc'], 'observed-now')
        self.assertNotIn('completed_at_utc', event)


if __name__ == '__main__':
    unittest.main(verbosity=2)
