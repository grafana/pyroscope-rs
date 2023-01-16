import hashlib
import os
import signal
import threading
import logging
import time
import traceback

import pyroscope

import uuid
try:
    from urllib.request import Request, urlopen
except ImportError:
    from urllib2 import Request, urlopen

token = os.getenv("PYROSCOPE_API_TOKEN")
app_name = 'pyroscopers.python.test'

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
        try:
            print(u)
            req = Request(u)
            req.add_header('Authorization', 'Bearer {}'.format(token))
            with urlopen(req) as response:
                code = response.getcode()
                # print(code)
                # print(response)
                # print(dir(response))
                body = response.read()
                print(body)
                if code == 200 and body != b'' and b'multihash' in body:
                    return
        except Exception:
            traceback.print_exc()
            continue


def do_one_test(on_cpu, gil_only, detect_subprocesses):
    p = os.fork()
    if p != 0:
        return p
    canary = uuid.uuid4().hex
    print('canary {}'.format(canary))
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

    signal.alarm(30)

    wait_render(canary)

    pyroscope.shutdown()
    exit(0)



if __name__ == '__main__':
    logger = logging.getLogger()
    logger.setLevel(logging.INFO)
    pids = []
    for on_cpu in [True, False]:
        for gil_only in [True, False]:
            for detect_subprocesses in [True, False]:
                pid = do_one_test(on_cpu, gil_only, detect_subprocesses)
                pids.append((pid, 'on_cpu {} gil_only {}  detect_subprocesses {}'.format(on_cpu, gil_only, detect_subprocesses)))
    res = []
    for testcase in pids:
        pid = testcase[0]
        test_name = testcase[1]
        _, exitcode = os.waitpid(pid, 0)
        print('pid {} {} exited with {}'.format(pid, test_name, exitcode))
        res.append((pid, exitcode))
    for testcase in res:
        pid = testcase[0]
        exitcode = testcase[1]
        if exitcode != 0:
            print('testcase {} failed'.format(pid))
            exit(1)

