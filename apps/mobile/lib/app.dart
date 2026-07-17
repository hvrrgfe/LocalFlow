import 'package:flutter/material.dart';
import 'pages/mod.dart';

class LocalFlowApp extends StatelessWidget {
  const LocalFlowApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'LocalFlow',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorSchemeSeed: const Color(0xFF4361EE),
        useMaterial3: true,
        appBarTheme: const AppBarTheme(centerTitle: true, elevation: 0),
      ),
      initialRoute: '/agents',
      onGenerateRoute: (settings) {
        // Parse routes with parameters
        final uri = Uri.tryParse(settings.name ?? '');
        if (uri == null) {
          return MaterialPageRoute(
            builder: (_) => const AgentListPage(),
            settings: settings,
          );
        }

        final path = uri.path;
        final segments = path.split('/').where((s) => s.isNotEmpty).toList();

        if (segments.length == 2 && segments[0] == 'agents') {
          if (segments[1] == 'new') {
            return MaterialPageRoute(
              builder: (_) => const AgentEditorPage(),
              settings: settings,
            );
          }
          return MaterialPageRoute(
            builder: (_) => AgentEditorPage(agentId: segments[1]),
            settings: settings,
          );
        }

        if (segments.length == 3 && segments[0] == 'agents' && segments[2] == 'workflow') {
          return MaterialPageRoute(
            builder: (_) => WorkflowEditorPage(agentId: segments[1]),
            settings: settings,
          );
        }

        if (segments.length == 2 && segments[0] == 'runs') {
          return MaterialPageRoute(
            builder: (_) => RunLogsPage(workflowId: segments[1]),
            settings: settings,
          );
        }

        // Top-level routes
        switch (path) {
          case '/agents':
            return MaterialPageRoute(builder: (_) => const AgentListPage(), settings: settings);
          case '/providers':
            return MaterialPageRoute(builder: (_) => const ApiManagementPage(), settings: settings);
          case '/runs':
            return MaterialPageRoute(builder: (_) => const RunLogsPage(), settings: settings);
          case '/security':
            return MaterialPageRoute(builder: (_) => const SecurityAuditPage(), settings: settings);
          default:
            return MaterialPageRoute(builder: (_) => const AgentListPage(), settings: settings);
        }
      },
    );
  }
}