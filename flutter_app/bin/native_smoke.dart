// bin/native_smoke.dart
// ------------------------------------------------------------
// Doc-engine 桌面端端到端冒烟（dart:ffi 调 doc_engine.dll/.dylib/.so）
//
// 用法（从仓库根目录）：
//   dart run flutter_app/bin/native_smoke.dart [main_tex_path]
//
// 流程：
//   1. 读 examples/paper3/upload.zip
//   2. 调 NativeBridge.convertZipToDocx
//   3. 写 examples/paper3/output/desktop-main-jos.docx
//   4. 断言：>= 4 KiB + "PK\x03\x04" 头
// ------------------------------------------------------------
import 'dart:io';
import 'package:doc_engine/native_bridge.dart';

Future<int> main(List<String> args) async {
  final repoRoot = Directory.current.path;
  final sep = Platform.pathSeparator;
  final zipPath = '$repoRoot${sep}examples${sep}paper3${sep}upload.zip';
  final outDir = '$repoRoot${sep}examples${sep}paper3${sep}output';
  final mainTex = args.isNotEmpty ? args[0] : 'main-jos.tex';

  // 允许通过环境变量指定 .dll 路径（CI / 桌面联调时拷到 build 目录）
  final libOverride = Platform.environment['DOC_ENGINE_LIB'];
  if (libOverride != null && libOverride.isNotEmpty) {
    stderr.writeln('[native-smoke] DOC_ENGINE_LIB=$libOverride');
  }

  final zipFile = File(zipPath);
  if (!zipFile.existsSync()) {
    stderr.writeln('[native-smoke] missing fixture: $zipPath');
    return 2;
  }
  final outDirDir = Directory(outDir);
  if (!outDirDir.existsSync()) {
    outDirDir.createSync(recursive: true);
  }

  stderr.writeln('[native-smoke] reading $zipPath');
  final zipBytes = await zipFile.readAsBytes();
  stderr.writeln('[native-smoke] zip bytes = ${zipBytes.length}');

  try {
    await NativeBridge.instance.ensureReady();
  } on NativeBridgeException catch (e) {
    stderr.writeln('[native-smoke] init failed: $e');
    return 3;
  }
  stderr.writeln('[native-smoke] library version = ${NativeBridge.instance.version}');

  final result = await NativeBridge.instance.convertZipToDocx(zipBytes, mainTex);
  final docx = result.docx;
  stderr.writeln('[native-smoke] docx bytes = ${docx.length}, warnings = ${result.warnings.length}');

  if (docx.length < 4 * 1024) {
    stderr.writeln('[native-smoke] FAIL: docx too small');
    return 4;
  }
  if (docx.length >= 4 &&
      !(docx[0] == 0x50 && docx[1] == 0x4B && docx[2] == 0x03 && docx[3] == 0x04)) {
    stderr.writeln('[native-smoke] FAIL: docx magic mismatch');
    return 5;
  }

  final outFile = File('$outDir${sep}desktop-main-jos.docx');
  await outFile.writeAsBytes(docx);
  stderr.writeln('[native-smoke] wrote $outFile (${docx.length} bytes)');
  return 0;
}
