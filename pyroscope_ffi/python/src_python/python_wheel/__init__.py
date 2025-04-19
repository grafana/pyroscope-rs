import threading
import warnings
import logging

from python_wheel import python_wheel

from contextlib import contextmanager

LOGGER = logging.getLogger(__name__)

LineNo = python_wheel.LineNo


def configure(
        app_name=None,  # todo remove
        application_name=None,
        server_address="http://localhost:4040",
        auth_token="",  # todo remove
        basic_auth_username="",
        basic_auth_password="",
        enable_logging=False,
        sample_rate=100,
        detect_subprocesses=False,  # todo remove
        oncpu=True,
        native=None,  # todo remove
        gil_only=True,
        report_pid=False,
        report_thread_id=False,
        report_thread_name=False,
        tags: dict[str, str] = None,
        tenant_id="",
        http_headers: dict[str, str] = None,
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
    return python_wheel.drop_agent()


def add_thread_tag(thread_id, key: str, value: str):
    python_wheel.add_thread_tag(thread_id, key, value)


def remove_thread_tag(thread_id, key: str, value: str):
    python_wheel.remove_thread_tag(thread_id, key, value)


def add_global_tag(key: str, value: str):
    python_wheel.add_global_tag(key, value)


def remove_global_tag(key: str, value: str):
    python_wheel.remove_global_tag(key, value)


@contextmanager
def tag_wrapper(tags: dict[str:str]):
    for key, value in tags.items():
        python_wheel.add_thread_tag(threading.get_ident(), key, value)
    try:
        yield
    finally:
        for key, value in tags.items():
            python_wheel.remove_thread_tag(threading.get_ident(), key, value)


# todo remove these
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
