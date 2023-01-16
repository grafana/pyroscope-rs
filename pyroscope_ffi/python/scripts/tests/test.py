import hashlib
import os
import signal
import threading
import logging
import time
import traceback
import sys
import multiprocessing

import pyroscope

import uuid

try:
    from urllib.request import Request, urlopen
except ImportError:
    from urllib2 import Request, urlopen

token = os.getenv("PYROSCOPE_API_TOKEN")
app_name = 'pyroscopers.python.test'
logger = logging.getLogger()


def hash(string):
    string = string.encode()
    string = hashlib.sha256(string).hexdigest()

    return string


def multihash(string):
    for i in range(0, 25510000):
        string = hash(string)
    return string


def multihash2(string):
    for i in range(0, 25510000):
        string = hash(string)
    return string


def wait_render(canary):
    while True:
        time.sleep(2)
        u = 'https://pyroscope.cloud/render?from=now-1h&until=now&format=collapsed&query=' \
            + '{}.cpu%7Bcanary%3D%22{}%22%7D'.format(app_name, canary)
        response = None
        try:
            logging.info('render %s', u)
            req = Request(u)
            req.add_header('Authorization', 'Bearer {}'.format(token))
            response = urlopen(req)
            code = response.getcode()
            body = response.read()
            logging.info("render body %s", body.decode('utf-8'))
            if code == 200 and body != b'' and b'multihash' in body:
                return
        except Exception:
            if response is not None:
                response.close()
            traceback.print_exc()
            continue


def do_one_test(on_cpu, gil_only, detect_subprocesses):
    canary = uuid.uuid4().hex
    logging.info('canary %s', canary)
    runid = os.getenv("PYROSCOPE_RUN_ID")
    pyroscope.configure(
        application_name=app_name,
        server_address="https://ingest.pyroscope.cloud",
        auth_token='{}'.format(token),
        enable_logging=True,
        detect_subprocesses=detect_subprocesses,
        oncpu=on_cpu,
        gil_only=gil_only,
        report_pid=True,
        report_thread_id=True,
        report_thread_name=True,

        tags={
            "detect_subprocesses": '{}'.format(detect_subprocesses),
            "oncpu": '{}'.format(on_cpu),
            "gil_only": '{}'.format(gil_only),
            "version": '{}'.format(os.getenv("PYTHON_VERSION")),
            "arch": '{}'.format(os.getenv("PYROSCOPE_ARCH")),
            "canary": canary,
            "run_id": runid,
        }
    )

    thread_1 = threading.Thread(target=multihash, args=('abc',))
    thread_2 = threading.Thread(target=multihash2, args=('abc',))

    thread_1.start()
    thread_2.start()

    signal.alarm(120)

    wait_render(canary)

    pyroscope.shutdown()
    exit(0)


if __name__ == '__main__':
    do_multiprocessing = True
    logger.setLevel(logging.INFO)
    if do_multiprocessing:
        res = []
        for on_cpu in [True, False]:
            for gil_only in [True, False]:
                for detect_subprocesses in [True, False]:
                    multiprocessing.log_to_stderr(logging.INFO)
                    p = multiprocessing.Process(target=do_one_test, args=(False, False, False))
                    p.start()
                    p.join()
                    res.append((p.exitcode, "{} {} {}".format(on_cpu, gil_only, detect_subprocesses)))
        for t in res:
            logging.Info("%s", str(t))
        for t in res:
            if t[0] != 0:
                logging.Error("test failed %s", str(t))
    else:
        on_cpu = sys.argv[1] == "true"
        gil_only = sys.argv[2] == "true"
        detect_subprocesses = sys.argv[3] == "true"
        do_one_test(on_cpu, gil_only, detect_subprocesses)
