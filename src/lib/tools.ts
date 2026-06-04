// 工具注册表：集中声明元数据与视图组件，新增工具只改这一处。
import type { Component } from "svelte";
import Timestamp from "./tools/Timestamp.svelte";
import Cron from "./tools/Cron.svelte";
import HexFloat from "./tools/HexFloat.svelte";
import BaseConverter from "./tools/BaseConverter.svelte";
import Json from "./tools/Json.svelte";
import Encoding from "./tools/Encoding.svelte";
import Regex from "./tools/Regex.svelte";
import Naming from "./tools/Naming.svelte";
import TextDiff from "./tools/TextDiff.svelte";
import ZhConvert from "./tools/ZhConvert.svelte";
import JsonStruct from "./tools/JsonStruct.svelte";
import Subnet from "./tools/Subnet.svelte";
import LocalIp from "./tools/LocalIp.svelte";
import PublicIp from "./tools/PublicIp.svelte";
import DnsLookup from "./tools/DnsLookup.svelte";
import Password from "./tools/Password.svelte";
import Crypto from "./tools/Crypto.svelte";
import RsaKeys from "./tools/RsaKeys.svelte";
import Hashing from "./tools/Hashing.svelte";

export interface ToolDef {
  id: string;
  name: string;
  desc: string;
  category: string;
  icon: string;
  component: Component;
}

export const tools: ToolDef[] = [
  {
    id: "timestamp",
    name: "时间戳转换",
    desc: "自动识别 秒/毫秒/微秒/纳秒，按时区换算，支持反向。",
    category: "时间",
    icon: "clock",
    component: Timestamp,
  },
  {
    id: "cron",
    name: "Cron 表达式",
    desc: "解析 5/6 字段 cron，预测未来运行时间。",
    category: "时间",
    icon: "calendar",
    component: Cron,
  },
  {
    id: "hexfloat",
    name: "Hex / Float",
    desc: "IEEE 754 位模式与浮点数互转（32 / 64 位）。",
    category: "数值",
    icon: "chip",
    component: HexFloat,
  },
  {
    id: "base",
    name: "进制转换",
    desc: "二 / 八 / 十 / 十六进制互转，含位宽与有无符号视图。",
    category: "数值",
    icon: "swap",
    component: BaseConverter,
  },
  {
    id: "json",
    name: "JSON 工具",
    desc: "格式化、压缩与校验（错误带行列定位）。",
    category: "文本",
    icon: "braces",
    component: Json,
  },
  {
    id: "encoding",
    name: "Base64 / URL",
    desc: "Base64（标准 / URL-safe）与 URL 百分号编解码。",
    category: "文本",
    icon: "code",
    component: Encoding,
  },
  {
    id: "regex",
    name: "正则测试器",
    desc: "实时匹配、捕获组与替换预览。",
    category: "文本",
    icon: "asterisk",
    component: Regex,
  },
  {
    id: "naming",
    name: "命名风格转换",
    desc: "camel / Pascal / snake / kebab 等风格互转。",
    category: "文本",
    icon: "type",
    component: Naming,
  },
  {
    id: "diff",
    name: "文本 Diff",
    desc: "按行比较两段文本，高亮新增与删除。",
    category: "文本",
    icon: "columns",
    component: TextDiff,
  },
  {
    id: "zhconvert",
    name: "简繁转换",
    desc: "词组级简繁/地区词转换（简体/繁体/台湾/香港）。",
    category: "文本",
    icon: "languages",
    component: ZhConvert,
  },
  {
    id: "jsonstruct",
    name: "JSON 转结构",
    desc: "JSON 推断类型，生成 Go / TS / Rust 结构。",
    category: "数据",
    icon: "filecode",
    component: JsonStruct,
  },
  {
    id: "subnet",
    name: "子网计算器",
    desc: "CIDR / 掩码解析，网络/广播/可用范围，IPv4 与 IPv6。",
    category: "网络",
    icon: "network",
    component: Subnet,
  },
  {
    id: "local-ip",
    name: "内网 IP",
    desc: "枚举本机网卡地址与主用出口 IP。纯本地，不联网。",
    category: "网络",
    icon: "monitor",
    component: LocalIp,
  },
  {
    id: "public-ip",
    name: "公网 IP",
    desc: "检测当前公网出口 IP，端点可配置。需联网。",
    category: "网络",
    icon: "globe",
    component: PublicIp,
  },
  {
    id: "dns-lookup",
    name: "域名解析",
    desc: "DoH 查询 A/AAAA/CNAME/MX/TXT/NS，端点可配置。需联网。",
    category: "网络",
    icon: "server",
    component: DnsLookup,
  },
  {
    id: "password",
    name: "随机密码",
    desc: "自定义字符集/长度/数量/必含/排除，默认排除易混淆字符。",
    category: "安全",
    icon: "shuffle",
    component: Password,
  },
  {
    id: "crypto",
    name: "加密 / 解密",
    desc: "AES-256-GCM/CBC、ChaCha20-Poly1305、RSA-OAEP，加解密。",
    category: "安全",
    icon: "lock",
    component: Crypto,
  },
  {
    id: "rsa-keys",
    name: "RSA 密钥对",
    desc: "生成 RSA 公私钥对（2048/3072/4096），PEM 可复制。",
    category: "安全",
    icon: "key",
    component: RsaKeys,
  },
  {
    id: "hashing",
    name: "哈希 / UUID",
    desc: "MD5 / SHA1 / SHA256 / SHA512 与 UUID v4 生成。",
    category: "安全",
    icon: "hash",
    component: Hashing,
  },
];

export function findTool(id: string): ToolDef {
  return tools.find((t) => t.id === id) ?? tools[0];
}
