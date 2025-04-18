import threading
import warnings
import logging
import json
from enum import Enum

from python_wheel import python_wheel

from contextlib import contextmanager

LOGGER = logging.getLogger(__name__)


# class LineNo(Enum):
#     LastInstruction = 0
#     First = 1
#     NoLine = 2

def configure(
        app_name=None,
        application_name=None,
        server_address="http://localhost:4040",
        auth_token="",
        basic_auth_username="",
        basic_auth_password="",
        enable_logging=False,
        sample_rate=100,
        detect_subprocesses=False,
        oncpu=True,
        native=None,
        gil_only=True,
        report_pid=False,
        report_thread_id=False,
        report_thread_name=False,
        tags=None,
        tenant_id="",
        http_headers=None,
        line_no=python_wheel.LineNo.LastInstruction,
):

    if app_name is not None:
        warnings.warn("app_name is deprecated, use application_name", DeprecationWarning)
        application_name = app_name

    if native is not None:
        warnings.warn("native is deprecated and not supported", DeprecationWarning)

    LOGGER.disabled = not enable_logging
    if enable_logging:
        log_level = LOGGER.getEffectiveLevel()
        python_wheel.initialize_logging(log_level)


    python_wheel.initialize_agent(
        application_name,
        server_address,
        auth_token,
        basic_auth_username,
        basic_auth_password,
        sample_rate,
        detect_subprocesses,
        oncpu,
        gil_only,
        report_pid,
        report_thread_id,
        report_thread_name,
        tags,
        tenant_id,
        http_headers,
        line_no
    )

def shutdown():
    drop = python_wheel.drop_agent()

    if drop:
        LOGGER.info("Pyroscope Agent successfully shutdown")
    else:
        LOGGER.warning("Pyroscope Agent shutdown failed")

def add_thread_tag(thread_id, key, value):
    python_wheel.add_thread_tag(thread_id, key.encode("UTF-8"), value.encode("UTF-8"))

def remove_thread_tag(thread_id, key, value):
    python_wheel.remove_thread_tag(thread_id, key.encode("UTF-8"), value.encode("UTF-8"))

def add_global_tag(thread_id, key, value):
    python_wheel.add_global_tag(key.encode("UTF-8"), value.encode("UTF-8"))

def remove_global_tag(key, value):
    python_wheel.remove_global_tag(key.encode("UTF-8"), value.encode("UTF-8"))

@contextmanager
def tag_wrapper(tags):
    for key, value in tags.items():
        python_wheel.add_thread_tag(threading.get_ident(), key.encode("UTF-8"), value.encode("UTF-8"))
    try:
        yield
    finally:
        for key, value in tags.items():
            python_wheel.remove_thread_tag(threading.get_ident(), key.encode("UTF-8"), value.encode("UTF-8"))

def stop():
    warnings.warn("deprecated, no longer applicable", DeprecationWarning)
def change_name(name):
    warnings.warn("deprecated, no longer applicable", DeprecationWarning)
def tag(tags):
    warnings.warn("deprecated, use tag_wrapper function", DeprecationWarning)
def remove_tags(*keys):
    warnings.warn("deprecated, no longer applicable", DeprecationWarning)
def build_summary():
    warnings.warn("deprecated, no longer applicable", DeprecationWarning)
def test_logger():
    warnings.warn("deprecated, no longer applicable", DeprecationWarning)
