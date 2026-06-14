import 'package:flutter/material.dart';

class Todo {
  final String id;
  String title;
  bool isDone;

  Todo({required this.id, required this.title, this.isDone = false});
}

class TodoPage extends StatefulWidget {
  const TodoPage({super.key});

  @override
  State<TodoPage> createState() => _TodoPageState();
}

class _TodoPageState extends State<TodoPage> {
  final List<Todo> _todos = [
    Todo(id: '1', title: '学习 Flutter Widget 系统'),
    Todo(id: '2', title: '理解 StatefulWidget 生命周期'),
    Todo(id: '3', title: '练习布局：Row / Column / Stack'),
    Todo(id: '4', title: '尝试自定义一个 Widget', isDone: true),
  ];

  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _addTodo() {
    final text = _controller.text.trim();
    if (text.isEmpty) return;
    setState(() {
      _todos.add(Todo(id: DateTime.now().toString(), title: text));
    });
    _controller.clear();
  }

  void _toggleTodo(Todo todo) {
    setState(() => todo.isDone = !todo.isDone);
  }

  void _deleteTodo(Todo todo) {
    setState(() => _todos.remove(todo));
  }

  int get _doneCount => _todos.where((t) => t.isDone).length;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('待办清单'),
      ),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 输入区
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _controller,
                    decoration: const InputDecoration(
                      labelText: '添加新任务...',
                      border: OutlineInputBorder(),
                    ),
                    onSubmitted: (_) => _addTodo(),
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton(
                  onPressed: _addTodo,
                  child: const Padding(
                    padding: EdgeInsets.symmetric(vertical: 12),
                    child: Text('添加'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // 统计
            Text(
              '共 ${_todos.length} 条，已完成 $_doneCount 条',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
            const SizedBox(height: 16),

            // 列表
            Expanded(
              child: _todos.isEmpty
                  ? const _EmptyHint()
                  : ListView.separated(
                      itemCount: _todos.length,
                      separatorBuilder: (_, __) => const SizedBox(height: 8),
                      itemBuilder: (context, index) {
                        final todo = _todos[index];
                        return _TodoTile(
                          todo: todo,
                          onToggle: () => _toggleTodo(todo),
                          onDelete: () => _deleteTodo(todo),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TodoTile extends StatelessWidget {
  const _TodoTile({
    required this.todo,
    required this.onToggle,
    required this.onDelete,
  });

  final Todo todo;
  final VoidCallback onToggle;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Theme.of(context).colorScheme.surfaceContainer,
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onToggle,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          child: Row(
            children: [
              Checkbox(
                value: todo.isDone,
                onChanged: (_) => onToggle(),
              ),
              Expanded(
                child: Text(
                  todo.title,
                  style: TextStyle(
                    decoration:
                        todo.isDone ? TextDecoration.lineThrough : null,
                    color: todo.isDone
                        ? Theme.of(context).colorScheme.onSurfaceVariant
                        : null,
                  ),
                ),
              ),
              IconButton(
                icon: const Icon(Icons.delete_outline),
                color: Theme.of(context).colorScheme.error,
                onPressed: onDelete,
                tooltip: '删除',
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _EmptyHint extends StatelessWidget {
  const _EmptyHint();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.inbox_outlined,
            size: 72,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 12),
          Text(
            '还没有任务，快添加一条吧！',
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
          ),
        ],
      ),
    );
  }
}
