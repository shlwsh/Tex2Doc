import 'package:flutter/material.dart';

enum AppThemeTone { defaultTone, blue, green, purple, orange, dark }

extension AppThemeToneLabel on AppThemeTone {
  String get storageKey => switch (this) {
    AppThemeTone.defaultTone => 'default',
    AppThemeTone.blue => 'blue',
    AppThemeTone.green => 'green',
    AppThemeTone.purple => 'purple',
    AppThemeTone.orange => 'orange',
    AppThemeTone.dark => 'dark',
  };

  static AppThemeTone fromStorageKey(String? value) {
    return AppThemeTone.values.firstWhere(
      (tone) => tone.storageKey == value,
      orElse: () => AppThemeTone.defaultTone,
    );
  }
}

class AppSpacing {
  static const double xxs = 4;
  static const double xs = 8;
  static const double sm = 12;
  static const double md = 16;
  static const double lg = 24;
  static const double xl = 32;
  static const double xxl = 48;
}

class AppRadius {
  static const double sm = 6;
  static const double md = 8;
  static const double lg = 10;
}

class AppMotion {
  static const Duration fast = Duration(milliseconds: 120);
  static const Duration normal = Duration(milliseconds: 180);
  static const Curve curve = Curves.easeOutCubic;
}

class AppBreakpoints {
  static const double mobile = 720;
  static const double tablet = 1040;
}

class AppColorTokens extends ThemeExtension<AppColorTokens> {
  final Color success;
  final Color warning;
  final Color info;
  final Color divider;
  final Color disabledText;

  const AppColorTokens({
    required this.success,
    required this.warning,
    required this.info,
    required this.divider,
    required this.disabledText,
  });

  @override
  AppColorTokens copyWith({
    Color? success,
    Color? warning,
    Color? info,
    Color? divider,
    Color? disabledText,
  }) {
    return AppColorTokens(
      success: success ?? this.success,
      warning: warning ?? this.warning,
      info: info ?? this.info,
      divider: divider ?? this.divider,
      disabledText: disabledText ?? this.disabledText,
    );
  }

  @override
  AppColorTokens lerp(ThemeExtension<AppColorTokens>? other, double t) {
    if (other is! AppColorTokens) return this;
    return AppColorTokens(
      success: Color.lerp(success, other.success, t)!,
      warning: Color.lerp(warning, other.warning, t)!,
      info: Color.lerp(info, other.info, t)!,
      divider: Color.lerp(divider, other.divider, t)!,
      disabledText: Color.lerp(disabledText, other.disabledText, t)!,
    );
  }
}
