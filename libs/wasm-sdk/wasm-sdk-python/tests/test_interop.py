from __future__ import annotations

from flow_like_wasm_sdk.interop import FlowPath

RAW = "Übersicht (2)#1.pdf"
LISTED = "%C3%9Cbersicht (2)%231.pdf"


class TestFlowPathNames:
    def test_file_name_decodes_listed_key(self) -> None:
        fp = FlowPath(f"dir/{LISTED}", "s")
        assert fp.file_name() == RAW
        assert fp.extension() == "pdf"

    def test_file_name_keeps_raw_and_malformed(self) -> None:
        assert FlowPath(f"dir/{RAW}", "s").file_name() == RAW
        assert FlowPath("100%.txt", "s").file_name() == "100%.txt"
        assert FlowPath("bad%FF.txt", "s").file_name() == "bad%FF.txt"
        assert FlowPath("", "s").file_name() is None

    def test_child_keeps_segment_verbatim(self) -> None:
        assert FlowPath("dir", "s").child(RAW).path == f"dir/{RAW}"
        assert FlowPath("dir", "s").join(LISTED).path == f"dir/{LISTED}"
