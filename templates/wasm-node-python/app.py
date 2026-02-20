"""
WASM Component Model entry point for Flow-Like Python nodes.

This module implements the WIT world exports (get-node, run, get-abi-version)
and connects the WIT host imports to the SDK's HostBridge.

The WitHostBridge and WIT definition are provided by the SDK — this file only
wires the node module to the WIT world exports.

Usage:
    componentize-py -d <wit-path> -w flow-like-node componentize app -o build/node.wasm
"""

from __future__ import annotations

import json

import node as _node_mod
from flow_like_wasm_sdk import ABI_VERSION, Context
from flow_like_wasm_sdk.bridge import _make_bridge
from flow_like_wasm_sdk.host import set_host

_bridge = _make_bridge()
set_host(_bridge)


class WitWorld:
    def get_node(self) -> str:
        if hasattr(_node_mod, "get_definitions"):
            defs = _node_mod.get_definitions()
            return json.dumps([d.to_dict() for d in defs])
        elif hasattr(_node_mod, "get_definition"):
            defn = _node_mod.get_definition()
            return json.dumps([defn.to_dict()])
        raise RuntimeError("Node module must export get_definition() or get_definitions()")

    def run(self, input_json: str) -> str:
        ctx = Context.from_json(input_json, _bridge)
        result = _node_mod.run(ctx)
        return json.dumps(result.to_dict())

    def get_abi_version(self) -> int:
        return ABI_VERSION
