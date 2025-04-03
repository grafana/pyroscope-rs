import typing
import threading

from opentelemetry.sdk.trace import (
    Span,
    ReadableSpan,
    SpanProcessor,
)
from opentelemetry.context import Context

import pyroscope

PROFILE_ID_SPAN_ATTRIBUTE_KEY = 'pyroscope.profile.id'
PROFILE_ID_PYROSCOPE_TAG_KEY = 'span_id'

def _is_root_span(span: Span):
    return span.parent is None or span.parent.is_remote

def _get_span_id(span: Span):
    return format(span.context.span_id, "016x")

# A span processor that sets a common identifier in spans and profiling samples, so that they can be linked together.
class PyroscopeSpanProcessor(SpanProcessor):

    def on_start(
        self, span: Span, parent_context: typing.Optional[Context] = None
    ) -> None:
        if _is_root_span(span):
            span.set_attribute(PROFILE_ID_SPAN_ATTRIBUTE_KEY, format(span.context.span_id, "016x"))
            pyroscope.add_thread_tag(threading.get_ident(), PROFILE_ID_PYROSCOPE_TAG_KEY, _get_span_id(span))

    def on_end(self, span: ReadableSpan) -> None:
        if _is_root_span(span):
            pyroscope.remove_thread_tag(threading.get_ident(), PROFILE_ID_PYROSCOPE_TAG_KEY, _get_span_id(span))

    def shutdown(self) -> None:
        pass

    def force_flush(self, timeout_millis: int = 30000) -> bool:
        return True
