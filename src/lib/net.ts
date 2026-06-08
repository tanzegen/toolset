// HTTP / WebSocket 客户端的命令封装（与后端 net::http / net::ws 对应）。
import { invoke, Channel } from "@tauri-apps/api/core";

export interface KV {
  key: string;
  value: string;
}

export interface HttpReq {
  method: string;
  url: string;
  params: KV[];
  headers: KV[];
  bodyType: "none" | "json" | "raw" | "form";
  body: string;
  form: KV[];
  vars: KV[];
  followRedirects: boolean;
  timeoutMs: number;
  skipTlsVerify: boolean;
}

export interface HttpResp {
  status: number;
  statusText: string;
  headers: KV[];
  body: string;
  size: number;
  timeMs: number;
  finalUrl: string;
}

export interface CurlParsed {
  method: string;
  url: string;
  headers: KV[];
  body: string;
  user: string | null;
}

export type WsFrame =
  | { kind: "message"; text: boolean; data: string }
  | { kind: "status"; state: string; msg: string };

export const net = {
  httpSend: (req: HttpReq) => invoke<HttpResp>("http_send", { req }),
  curlParse: (text: string) => invoke<CurlParsed>("curl_parse", { text }),

  wsConnect: (url: string, headers: KV[], channel: Channel<WsFrame>) =>
    invoke<string>("ws_connect", { url, headers, channel }),
  wsSend: (sessionId: string, text: boolean, data: string) =>
    invoke<void>("ws_send", { sessionId, text, data }),
  wsPing: (sessionId: string) => invoke<void>("ws_ping", { sessionId }),
  wsClose: (sessionId: string) => invoke<void>("ws_close", { sessionId }),
};
