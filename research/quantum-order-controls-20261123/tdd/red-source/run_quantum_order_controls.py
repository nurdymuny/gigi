"""Behavioural RED stubs for the bounded existing-control runner."""


def verify_run(folder, replay=True):
    return {'status': 'PASS'}


def validate_cases(cases):
    return True


def supervise(process, folder):
    return 0
