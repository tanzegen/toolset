// 后端命令的强类型封装。Tauri 默认把 JS 的 camelCase 参数转为 Rust 的 snake_case。

import { invoke } from "@tauri-apps/api/core";

export interface TimestampResult {
  detected_unit: string;
  epoch_seconds: string;
  epoch_millis: string;
  epoch_micros: string;
  epoch_nanos: string;
  utc: string;
  local: string;
  iso8601: string;
  rfc2822: string;
  relative: string;
  weekday: string;
  timezone: string;
}

export interface FloatResult {
  width: number;
  float_value: string;
  hex: string;
  binary: string;
  sign: number;
  exponent_raw: number;
  exponent_unbiased: number;
  mantissa_hex: string;
  category: string;
  int_unsigned: string;
  int_signed: string;
}

export interface BaseResult {
  bin: string;
  oct: string;
  dec_unsigned: string;
  dec_signed: string;
  hex: string;
  bit_width: number;
  bits_grouped: string;
}

export interface HashResult {
  md5: string;
  sha1: string;
  sha256: string;
  sha512: string;
}

export interface JsonValidateResult {
  valid: boolean;
  message: string;
  line: number;
  column: number;
}

export interface CronField {
  label: string;
  value: string;
}
export interface CronRun {
  local: string;
  utc: string;
  relative: string;
}
export interface CronResult {
  fields: CronField[];
  next_runs: CronRun[];
  timezone: string;
}

export interface RegexGroup {
  name: string;
  value: string | null;
}
export interface RegexMatch {
  index: number;
  start: number;
  end: number;
  text: string;
  groups: RegexGroup[];
}
export interface RegexResult {
  matches: RegexMatch[];
  count: number;
  replaced: string | null;
}

export interface SubnetResult {
  version: number;
  cidr: string;
  address: string;
  network: string;
  broadcast: string;
  netmask: string;
  wildcard: string;
  prefix: number;
  first_host: string;
  last_host: string;
  total: string;
  usable: string;
  network_int: string;
  ip_class: string;
  is_private: boolean;
}

export interface CaseResult {
  lower_camel: string;
  upper_camel: string;
  snake: string;
  shouty_snake: string;
  kebab: string;
  shouty_kebab: string;
  train: string;
  title: string;
}

export interface DiffRow {
  tag: string;
  left_no: number | null;
  right_no: number | null;
  text: string;
}
export interface DiffResult {
  rows: DiffRow[];
  added: number;
  removed: number;
}

export interface IfaceAddr {
  name: string;
  ip: string;
  version: number;
  is_loopback: boolean;
}
export interface LocalIpResult {
  primary: string | null;
  interfaces: IfaceAddr[];
}

export const api = {
  timestampConvert: (input: string, unit: string, tz: string) =>
    invoke<TimestampResult>("timestamp_convert", { input, unit, tz }),
  timestampFromDatetime: (datetime: string, tz: string) =>
    invoke<TimestampResult>("timestamp_from_datetime", { datetime, tz }),
  nowMillis: () => invoke<string>("now_millis"),
  listTimezones: () => invoke<string[]>("list_timezones"),

  hexToFloat: (hex: string, width: string) =>
    invoke<FloatResult>("hex_to_float", { hex, width }),
  floatToHex: (value: string, width: string) =>
    invoke<FloatResult>("float_to_hex", { value, width }),
  baseConvert: (input: string, fromBase: number, bitWidth: number) =>
    invoke<BaseResult>("base_convert", { input, fromBase, bitWidth }),

  base64Encode: (input: string, urlSafe: boolean) =>
    invoke<string>("base64_encode", { input, urlSafe }),
  base64Decode: (input: string, urlSafe: boolean) =>
    invoke<string>("base64_decode", { input, urlSafe }),
  urlEncode: (input: string) => invoke<string>("url_encode", { input }),
  urlDecode: (input: string) => invoke<string>("url_decode", { input }),

  jsonFormat: (input: string, indent: number) =>
    invoke<string>("json_format", { input, indent }),
  jsonMinify: (input: string) => invoke<string>("json_minify", { input }),
  jsonValidate: (input: string) =>
    invoke<JsonValidateResult>("json_validate", { input }),

  hashText: (input: string) => invoke<HashResult>("hash_text", { input }),
  uuidV4: (count: number) => invoke<string[]>("uuid_v4", { count }),

  cronExplain: (expr: string, tz: string, count: number) =>
    invoke<CronResult>("cron_explain", { expr, tz, count }),
  regexTest: (
    pattern: string,
    flags: string,
    text: string,
    replacement: string | null,
  ) => invoke<RegexResult>("regex_test", { pattern, flags, text, replacement }),
  subnetCalc: (input: string) => invoke<SubnetResult>("subnet_calc", { input }),
  jsonToStruct: (json: string, lang: string, rootName: string) =>
    invoke<string>("json_to_struct", { json, lang, rootName }),
  convertCase: (input: string) => invoke<CaseResult>("convert_case", { input }),
  textDiff: (left: string, right: string) =>
    invoke<DiffResult>("text_diff", { left, right }),
  localIps: () => invoke<LocalIpResult>("local_ips"),
};

/** 把 invoke 抛出的错误（后端序列化的字符串）规整为消息文本。 */
export function errMsg(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as any).message);
  return String(e);
}
