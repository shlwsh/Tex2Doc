import 'package:flutter/material.dart';

import 'app_tokens.dart';

class AppTheme {
  static ThemeData light(AppThemeTone tone) {
    return _buildTheme(tone: tone, brightness: Brightness.light);
  }

  static ThemeData dark(AppThemeTone tone) {
    return _buildTheme(
      tone: tone == AppThemeTone.defaultTone ? AppThemeTone.dark : tone,
      brightness: Brightness.dark,
    );
  }

  static ThemeData _buildTheme({
    required AppThemeTone tone,
    required Brightness brightness,
  }) {
    final seed = _seedColor(tone);
    final colorScheme = ColorScheme.fromSeed(
      seedColor: seed,
      brightness: brightness,
    );
    final isDark = brightness == Brightness.dark;
    final surface = isDark ? const Color(0xFF111827) : const Color(0xFFFFFFFF);
    final background = isDark
        ? const Color(0xFF0B1120)
        : const Color(0xFFF6F8FB);
    final border = isDark ? const Color(0xFF334155) : const Color(0xFFD8DEE8);

    return ThemeData(
      useMaterial3: true,
      brightness: brightness,
      colorScheme: colorScheme.copyWith(
        surface: surface,
        surfaceContainerHighest: isDark
            ? const Color(0xFF182235)
            : const Color(0xFFF1F5F9),
        outline: border,
      ),
      scaffoldBackgroundColor: background,
      fontFamily: 'Inter',
      textTheme: _textTheme(isDark),
      cardTheme: CardThemeData(
        elevation: 0,
        color: surface,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
          side: BorderSide(color: border),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: surface,
        isDense: true,
        contentPadding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.md,
          vertical: AppSpacing.sm,
        ),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
          borderSide: BorderSide(color: border),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(AppRadius.md),
          borderSide: BorderSide(color: border),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          minimumSize: const Size(96, 44),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.md),
          ),
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(96, 44),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppRadius.md),
          ),
          side: BorderSide(color: border),
          textStyle: const TextStyle(fontWeight: FontWeight.w600),
        ),
      ),
      extensions: [
        AppColorTokens(
          success: isDark ? const Color(0xFF34D399) : const Color(0xFF0F8A5F),
          warning: isDark ? const Color(0xFFFBBF24) : const Color(0xFFB7791F),
          info: isDark ? const Color(0xFF60A5FA) : const Color(0xFF2563EB),
          divider: border,
          disabledText: isDark
              ? const Color(0xFF64748B)
              : const Color(0xFF94A3B8),
        ),
      ],
    );
  }

  static TextTheme _textTheme(bool isDark) {
    final textColor = isDark
        ? const Color(0xFFF8FAFC)
        : const Color(0xFF172033);
    final secondary = isDark
        ? const Color(0xFFCBD5E1)
        : const Color(0xFF42526B);
    return TextTheme(
      displaySmall: TextStyle(
        fontSize: 32,
        fontWeight: FontWeight.w700,
        color: textColor,
      ),
      titleLarge: TextStyle(
        fontSize: 22,
        fontWeight: FontWeight.w700,
        color: textColor,
      ),
      titleMedium: TextStyle(
        fontSize: 18,
        fontWeight: FontWeight.w700,
        color: textColor,
      ),
      bodyLarge: TextStyle(fontSize: 16, color: textColor),
      bodyMedium: TextStyle(fontSize: 14, color: textColor),
      bodySmall: TextStyle(fontSize: 12, color: secondary),
      labelLarge: TextStyle(
        fontSize: 14,
        fontWeight: FontWeight.w600,
        color: textColor,
      ),
    );
  }

  static Color _seedColor(AppThemeTone tone) {
    return switch (tone) {
      AppThemeTone.defaultTone => const Color(0xFF2563EB),
      AppThemeTone.blue => const Color(0xFF1D4ED8),
      AppThemeTone.green => const Color(0xFF047857),
      AppThemeTone.purple => const Color(0xFF7C3AED),
      AppThemeTone.orange => const Color(0xFFEA580C),
      AppThemeTone.dark => const Color(0xFF60A5FA),
    };
  }
}
