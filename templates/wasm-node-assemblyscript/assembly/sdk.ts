/**
 * Flow-Like WASM SDK for AssemblyScript
 *
 * This module provides types and utilities for building WASM nodes in AssemblyScript.
 * Uses manual JSON serialization to avoid external dependencies.
 */

// ============================================================================
// JSON Utilities
// ============================================================================

export function escapeJsonString(s: string): string {
  let result = "";
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c == 34) {
      result += '\\"';
    } else if (c == 92) {
      result += "\\\\";
    } else if (c == 10) {
      result += "\\n";
    } else if (c == 13) {
      result += "\\r";
    } else if (c == 9) {
      result += "\\t";
    } else if (c < 32) {
      const hex = c.toString(16);
      result += "\\u" + "0000".substr(0, 4 - hex.length) + hex;
    } else {
      result += String.fromCharCode(c);
    }
  }
  return result;
}

export function jsonString(s: string): string {
  return '"' + escapeJsonString(s) + '"';
}

// ============================================================================
// ABI Types
// ============================================================================

export const ABI_VERSION: i32 = 1;

/** Node quality scores */
export class NodeScores {
  privacy: u8 = 0;
  security: u8 = 0;
  performance: u8 = 0;
  governance: u8 = 0;
  reliability: u8 = 0;
  cost: u8 = 0;

  toJSON(): string {
    return `{"privacy":${this.privacy},"security":${this.security},"performance":${this.performance},"governance":${this.governance},"reliability":${this.reliability},"cost":${this.cost}}`;
  }
}

/** Pin definition for node inputs/outputs */
export class PinDefinition {
  name = "";
  friendly_name = "";
  description = "";
  pin_type = "";
  data_type = "";
  default_value: string | null = null;
  value_type: string | null = null;
  schema: string | null = null;

  static input(name: string, friendlyName: string, description: string, dataType: string): PinDefinition {
    const pin = new PinDefinition();
    pin.name = name;
    pin.friendly_name = friendlyName;
    pin.description = description;
    pin.pin_type = "Input";
    pin.data_type = dataType;
    return pin;
  }

  static output(name: string, friendlyName: string, description: string, dataType: string): PinDefinition {
    const pin = new PinDefinition();
    pin.name = name;
    pin.friendly_name = friendlyName;
    pin.description = description;
    pin.pin_type = "Output";
    pin.data_type = dataType;
    return pin;
  }

  withDefault(value: string): PinDefinition {
    this.default_value = value;
    return this;
  }

  toJSON(): string {
    let json = `{"name":${jsonString(this.name)},"friendly_name":${jsonString(this.friendly_name)},"description":${jsonString(this.description)},"pin_type":"${this.pin_type}","data_type":"${this.data_type}"`;
    if (this.default_value !== null) {
      json += `,"default_value":${this.default_value}`;
    }
    if (this.value_type !== null) {
      json += `,"value_type":${jsonString(this.value_type!)}`;
    }
    if (this.schema !== null) {
      json += `,"schema":${jsonString(this.schema!)}`;
    }
    json += "}";
    return json;
  }
}

/** Node definition */
export class NodeDefinition {
  name = "";
  friendly_name = "";
  description = "";
  category = "";
  icon: string | null = null;
  pins: PinDefinition[] = [];
  scores: NodeScores | null = null;
  long_running: bool = false;
  docs: string | null = null;
  abi_version: i32 = ABI_VERSION;

  addPin(pin: PinDefinition): NodeDefinition {
    this.pins.push(pin);
    return this;
  }

  setScores(scores: NodeScores): NodeDefinition {
    this.scores = scores;
    return this;
  }

  toJSON(): string {
    let pinsJson = "[";
    for (let i = 0; i < this.pins.length; i++) {
      if (i > 0) pinsJson += ",";
      pinsJson += this.pins[i].toJSON();
    }
    pinsJson += "]";

    let json = `{"name":${jsonString(this.name)},"friendly_name":${jsonString(this.friendly_name)},"description":${jsonString(this.description)},"category":${jsonString(this.category)},"pins":${pinsJson},"long_running":${this.long_running},"abi_version":${this.abi_version}`;

    if (this.icon !== null) {
      json += `,"icon":${jsonString(this.icon!)}`;
    }
    if (this.scores !== null) {
      json += `,"scores":${this.scores!.toJSON()}`;
    }
    if (this.docs !== null) {
      json += `,"docs":${jsonString(this.docs!)}`;
    }
    json += "}";
    return json;
  }
}

/** Execution input from host */
export class ExecutionInput {
  inputs: Map<string, string> = new Map();
  node_id = "";
  run_id = "";
  app_id = "";
  board_id = "";
  user_id = "";
  stream_state: bool = false;
  log_level: u8 = 1;
}

/** Execution result to return to host */
export class ExecutionResult {
  outputs: Map<string, string> = new Map();
  error: string | null = null;
  activate_exec: string[] = [];
  pending: bool = false;

  static success(): ExecutionResult {
    return new ExecutionResult();
  }

  static fail(message: string): ExecutionResult {
    const result = new ExecutionResult();
    result.error = message;
    return result;
  }

  setOutput(name: string, value: string): ExecutionResult {
    this.outputs.set(name, value);
    return this;
  }

  activateExec(pinName: string): ExecutionResult {
    this.activate_exec.push(pinName);
    return this;
  }

  setPending(pending: bool): ExecutionResult {
    this.pending = pending;
    return this;
  }

  toJSON(): string {
    let outputsJson = "{";
    const keys = this.outputs.keys();
    for (let i = 0; i < keys.length; i++) {
      if (i > 0) outputsJson += ",";
      const key = keys[i];
      outputsJson += `${jsonString(key)}:${this.outputs.get(key)}`;
    }
    outputsJson += "}";

    let execJson = "[";
    for (let i = 0; i < this.activate_exec.length; i++) {
      if (i > 0) execJson += ",";
      execJson += jsonString(this.activate_exec[i]);
    }
    execJson += "]";

    let json = `{"outputs":${outputsJson},"activate_exec":${execJson},"pending":${this.pending}`;
    if (this.error !== null) {
      json += `,"error":${jsonString(this.error!)}`;
    }
    json += "}";
    return json;
  }
}

// ============================================================================
// Host Function Imports
// ============================================================================

@external("env", "host_log")
declare function host_log(level: i32, ptr: i32, len: i32): void;

@external("env", "host_stream")
declare function host_stream(eventTypePtr: i32, eventTypeLen: i32, dataPtr: i32, dataLen: i32): void;

@external("env", "host_get_variable")
declare function host_get_variable(namePtr: i32, nameLen: i32): i64;

@external("env", "host_set_variable")
declare function host_set_variable(namePtr: i32, nameLen: i32, valuePtr: i32, valueLen: i32): i32;

@external("env", "host_time_now")
declare function host_time_now(): i64;

@external("env", "host_random")
declare function host_random(): i64;

// ============================================================================
// Log Functions
// ============================================================================

export enum LogLevel {
  Debug = 0,
  Info = 1,
  Warn = 2,
  Error = 3,
  Fatal = 4,
}

function logMessage(level: LogLevel, message: string): void {
  const buf = String.UTF8.encode(message);
  host_log(level, changetype<i32>(buf), buf.byteLength);
}

export function debug(message: string): void {
  logMessage(LogLevel.Debug, message);
}

export function info(message: string): void {
  logMessage(LogLevel.Info, message);
}

export function warn(message: string): void {
  logMessage(LogLevel.Warn, message);
}

export function error(message: string): void {
  logMessage(LogLevel.Error, message);
}

// ============================================================================
// Stream Functions
// ============================================================================

export function stream(eventType: string, data: string): void {
  const typeBuf = String.UTF8.encode(eventType);
  const dataBuf = String.UTF8.encode(data);
  host_stream(
    changetype<i32>(typeBuf),
    typeBuf.byteLength,
    changetype<i32>(dataBuf),
    dataBuf.byteLength
  );
}

export function streamText(text: string): void {
  stream("wasm_text", text);
}

export function streamProgress(progress: f32, message: string): void {
  stream("wasm_progress", `{"progress":${progress},"message":"${message}"}`);
}

// ============================================================================
// Utility Functions
// ============================================================================

export function now(): i64 {
  return host_time_now();
}

export function random(): i64 {
  return host_random();
}

// ============================================================================
// Memory Exports
// ============================================================================

let resultBuffer: ArrayBuffer = new ArrayBuffer(0);

export function alloc(size: i32): i32 {
  const buf = new ArrayBuffer(size);
  return changetype<i32>(buf);
}

export function dealloc(ptr: i32, size: i32): void {
  // Memory is managed by GC
}

export function get_abi_version(): i32 {
  return ABI_VERSION;
}

// ============================================================================
// Result Packing
// ============================================================================

export function packResult(json: string): i64 {
  const buf = String.UTF8.encode(json);
  resultBuffer = buf;
  const ptr = changetype<i32>(buf);
  const len = buf.byteLength;
  return (i64(ptr) << 32) | i64(len);
}

export function serializeDefinition(def: NodeDefinition): i64 {
  const json = def.toJSON();
  return packResult(json);
}

export function serializeResult(result: ExecutionResult): i64 {
  const json = result.toJSON();
  return packResult(json);
}

// ============================================================================
// JSON Parsing Helpers
// ============================================================================

function isWhitespace(c: i32): bool {
  return c == 32 || c == 9 || c == 10 || c == 13;
}

function extractStringValue(json: string, key: string): string {
  const keyStr = '"' + key + '"';
  const idx = json.indexOf(keyStr);
  if (idx < 0) return "";

  let i = idx + keyStr.length;
  while (i < json.length && (isWhitespace(json.charCodeAt(i)) || json.charCodeAt(i) == 58)) i++;

  if (json.charCodeAt(i) != 34) return "";
  i++;
  const start = i;
  while (i < json.length && json.charCodeAt(i) != 34) i++;
  return json.substring(start, i);
}

function extractBoolValue(json: string, key: string): bool {
  const keyStr = '"' + key + '"';
  const idx = json.indexOf(keyStr);
  if (idx < 0) return false;

  let i = idx + keyStr.length;
  while (i < json.length && (isWhitespace(json.charCodeAt(i)) || json.charCodeAt(i) == 58)) i++;

  return json.substr(i, 4) == "true";
}

function extractIntValue(json: string, key: string): i32 {
  const keyStr = '"' + key + '"';
  const idx = json.indexOf(keyStr);
  if (idx < 0) return 0;

  let i = idx + keyStr.length;
  while (i < json.length && (isWhitespace(json.charCodeAt(i)) || json.charCodeAt(i) == 58)) i++;

  let num = 0;
  let neg = false;
  if (json.charCodeAt(i) == 45) {
    neg = true;
    i++;
  }
  while (i < json.length) {
    const c = json.charCodeAt(i);
    if (c < 48 || c > 57) break;
    num = num * 10 + (c - 48);
    i++;
  }
  return neg ? -num : num;
}

function parseInputsObject(json: string, map: Map<string, string>): void {
  let i = 1;
  while (i < json.length - 1) {
    while (i < json.length && isWhitespace(json.charCodeAt(i))) i++;
    if (i >= json.length - 1 || json.charCodeAt(i) == 125) break;

    if (json.charCodeAt(i) != 34) {
      i++;
      continue;
    }
    const keyStart = i + 1;
    i++;
    while (i < json.length && json.charCodeAt(i) != 34) i++;
    const key = json.substring(keyStart, i);
    i++;

    while (i < json.length && (isWhitespace(json.charCodeAt(i)) || json.charCodeAt(i) == 58)) i++;

    const valueStart = i;
    if (json.charCodeAt(i) == 34) {
      i++;
      while (i < json.length) {
        if (json.charCodeAt(i) == 34 && json.charCodeAt(i - 1) != 92) break;
        i++;
      }
      i++;
    } else if (json.charCodeAt(i) == 123) {
      let depth = 1;
      i++;
      while (depth > 0 && i < json.length) {
        if (json.charCodeAt(i) == 123) depth++;
        else if (json.charCodeAt(i) == 125) depth--;
        i++;
      }
    } else if (json.charCodeAt(i) == 91) {
      let depth = 1;
      i++;
      while (depth > 0 && i < json.length) {
        if (json.charCodeAt(i) == 91) depth++;
        else if (json.charCodeAt(i) == 93) depth--;
        i++;
      }
    } else {
      while (i < json.length && !isWhitespace(json.charCodeAt(i)) && json.charCodeAt(i) != 44 && json.charCodeAt(i) != 125) i++;
    }
    const value = json.substring(valueStart, i);
    map.set(key, value);

    while (i < json.length && (isWhitespace(json.charCodeAt(i)) || json.charCodeAt(i) == 44)) i++;
  }
}

function parseExecutionInputJson(json: string): ExecutionInput {
  const input = new ExecutionInput();

  const inputsStart = json.indexOf('"inputs"');
  if (inputsStart >= 0) {
    const objStart = json.indexOf("{", inputsStart + 8);
    if (objStart >= 0) {
      let depth = 1;
      let objEnd = objStart + 1;
      while (depth > 0 && objEnd < json.length) {
        const c = json.charCodeAt(objEnd);
        if (c == 123) depth++;
        else if (c == 125) depth--;
        objEnd++;
      }
      const inputsStr = json.substring(objStart, objEnd);
      parseInputsObject(inputsStr, input.inputs);
    }
  }

  input.node_id = extractStringValue(json, "node_id");
  input.run_id = extractStringValue(json, "run_id");
  input.app_id = extractStringValue(json, "app_id");
  input.board_id = extractStringValue(json, "board_id");
  input.user_id = extractStringValue(json, "user_id");
  input.stream_state = extractBoolValue(json, "stream_state");
  input.log_level = u8(extractIntValue(json, "log_level"));

  return input;
}

// ============================================================================
// Input Parsing
// ============================================================================

export function parseInput(ptr: i32, len: i32): ExecutionInput {
  const buf = new Uint8Array(len);
  memory.copy(changetype<i32>(buf.buffer), ptr, len);
  const json = String.UTF8.decode(buf.buffer);
  return parseExecutionInputJson(json);
}
