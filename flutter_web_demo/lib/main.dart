import 'package:flutter/material.dart';

import 'pages/home_page.dart';
import 'pages/counter_page.dart';
import 'pages/todo_page.dart';
import 'pages/about_page.dart';

void main() {
  runApp(const FlutterWebDemoApp());
}

class FlutterWebDemoApp extends StatelessWidget {
  const FlutterWebDemoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flutter Web 学习示例',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF0175C2),
          brightness: Brightness.light,
        ),
        useMaterial3: true,
        appBarTheme: const AppBarTheme(
          centerTitle: false,
          elevation: 0,
        ),
      ),
      darkTheme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF0175C2),
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      themeMode: ThemeMode.system,
      home: const AppShell(),
      routes: {
        '/counter': (context) => const CounterPage(),
        '/todo': (context) => const TodoPage(),
        '/about': (context) => const AboutPage(),
      },
    );
  }
}

class AppShell extends StatefulWidget {
  const AppShell({super.key});

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  int _selectedIndex = 0;

  static const _pages = <Widget>[
    HomePage(),
    CounterPage(),
    TodoPage(),
    AboutPage(),
  ];

  static const _navItems = <NavigationDestination>[
    NavigationDestination(icon: Icon(Icons.home), label: '首页'),
    NavigationDestination(icon: Icon(Icons.add_circle), label: '计数器'),
    NavigationDestination(icon: Icon(Icons.checklist), label: '待办清单'),
    NavigationDestination(icon: Icon(Icons.info), label: '关于'),
  ];

  void _onSelect(int index) {
    setState(() => _selectedIndex = index);
  }

  @override
  Widget build(BuildContext context) {
    final isWide = MediaQuery.of(context).size.width >= 720;

    return Scaffold(
      body: Row(
        children: [
          if (isWide) _buildDrawer(context, permanent: true),
          Expanded(child: _pages[_selectedIndex]),
        ],
      ),
      drawer: isWide ? null : _buildDrawer(context, permanent: false),
      bottomNavigationBar: isWide
          ? null
          : NavigationBar(
              selectedIndex: _selectedIndex,
              onDestinationSelected: _onSelect,
              destinations: _navItems,
            ),
    );
  }

  Widget _buildDrawer(BuildContext context, {required bool permanent}) {
    final colorScheme = Theme.of(context).colorScheme;
    final surface = permanent
        ? colorScheme.surfaceContainerHighest
        : colorScheme.surface;

    return NavigationDrawer(
      backgroundColor: surface,
      selectedIndex: _selectedIndex,
      onDestinationSelected: (index) {
        _onSelect(index);
        if (!permanent) Navigator.of(context).pop();
      },
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 24, 16, 8),
          child: Text(
            'Flutter Web Demo',
            style: Theme.of(context).textTheme.titleLarge,
          ),
        ),
        const Padding(
          padding: EdgeInsets.fromLTRB(28, 0, 24, 16),
          child: Text('一个极简的 Flutter Web 学习项目'),
        ),
        const Padding(
          padding: EdgeInsets.fromLTRB(28, 0, 24, 10),
          child: Divider(height: 1),
        ),
        ..._navItems.map(
          (item) => NavigationDrawerDestination(
            icon: item.icon,
            label: Text(item.label),
          ),
        ),
      ],
    );
  }
}
