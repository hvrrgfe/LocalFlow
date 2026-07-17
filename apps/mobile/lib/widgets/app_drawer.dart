import 'package:flutter/material.dart';

class AppDrawer extends StatelessWidget {
  final String currentRoute;

  const AppDrawer({super.key, required this.currentRoute});

  @override
  Widget build(BuildContext context) {
    return Drawer(
      child: ListView(
        padding: EdgeInsets.zero,
        children: [
          DrawerHeader(
            decoration: const BoxDecoration(color: Color(0xFF1A1A2E)),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                Text('LocalFlow',
                    style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                        color: Colors.white, fontWeight: FontWeight.bold)),
                const SizedBox(height: 4),
                Text('本地 AI 工作流',
                    style: TextStyle(color: Colors.white70, fontSize: 13)),
              ],
            ),
          ),
          _navItem(context, 'Agent 管理', '/agents', Icons.smart_toy_outlined),
          _navItem(context, 'API 管理', '/providers', Icons.key_outlined),
          _navItem(context, '运行日志', '/runs', Icons.history_outlined),
          _navItem(context, '安全审计', '/security', Icons.security_outlined),
        ],
      ),
    );
  }

  Widget _navItem(BuildContext context, String title, String route, IconData icon) {
    final isActive = currentRoute == route;
    return ListTile(
      leading: Icon(icon, color: isActive ? Colors.indigo : null),
      title: Text(title,
          style: TextStyle(
            fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
            color: isActive ? Colors.indigo : null,
          )),
      selected: isActive,
      onTap: () {
        Navigator.pop(context);
        if (!isActive) {
          Navigator.pushReplacementNamed(context, route);
        }
      },
    );
  }
}