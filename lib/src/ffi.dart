/// dart:ffi 桥接到 rich_text 原生核心（方案 B 单一事实源）。
///
/// 走 `rich_text_uniffi` cdylib 上我们自定义的扁平 C-ABI（[rt_parse_to_json] 等），
/// 不依赖 `uniffi-dart-bindgen`。原生库由宿主工程按平台打包：
/// - Android：放入 `jniLibs/<abi>/librich_text_uniffi.so`
/// - iOS：以 `rich_text_uniffi` framework 形式打包
/// 测试/桌面宿主可用 `RICH_TEXT_LIB` 环境变量指向 .so 绝对路径覆盖查找。
library;

import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

typedef _ParseNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _ParseDart = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _SerializeNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _SerializeDart = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

final DynamicLibrary _lib = _loadLibrary();

DynamicLibrary _loadLibrary() {
  final overridePath = Platform.environment['RICH_TEXT_LIB'];
  if (overridePath != null && overridePath.isNotEmpty) {
    return DynamicLibrary.open(overridePath);
  }
  if (Platform.isAndroid) return DynamicLibrary.open('librich_text_uniffi.so');
  if (Platform.isIOS) return DynamicLibrary.open('rich_text_uniffi');
  // Linux / macOS / Windows 宿主（测试）。
  return DynamicLibrary.open('librich_text_uniffi.so');
}

final _parseToJson =
    _lib.lookupFunction<_ParseNative, _ParseDart>('rt_parse_to_json');
final _serializeFromJson = _lib
    .lookupFunction<_SerializeNative, _SerializeDart>(
        'rt_serialize_blocks_from_json');
final _freeString =
    _lib.lookupFunction<_FreeNative, _FreeDart>('rt_free_string');

/// Markdown 子集 → 块树 JSON（Rust 核心）。返回的字符串由 Rust malloc，本函数负责释放。
String parseToJson(String markdown) {
  final inUtf8 = markdown.toNativeUtf8();
  final outPtr = _parseToJson(inUtf8);
  malloc.free(inUtf8);
  final out = outPtr.toDartString();
  _freeString(outPtr);
  return out;
}

/// 块树 JSON → Markdown（编辑器回写出口）。返回的字符串由 Rust malloc，本函数负责释放。
String serializeBlocksFromJson(String json) {
  final inUtf8 = json.toNativeUtf8();
  final outPtr = _serializeFromJson(inUtf8);
  malloc.free(inUtf8);
  final out = outPtr.toDartString();
  _freeString(outPtr);
  return out;
}
