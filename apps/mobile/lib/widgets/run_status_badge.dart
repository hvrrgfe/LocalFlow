import 'package:flutter/material.dart';
import '../models/mod.dart';

class RunStatusBadge extends StatelessWidget {
  final RunStatus status;

  const RunStatusBadge({super.key, required this.status});

  @override
  Widget build(BuildContext context) {
    final (Color color, String label) = switch (status) {
      RunStatus.succeeded => (Colors.green, '成功'),
      RunStatus.failed => (Colors.red, '失败'),
      RunStatus.running => (Colors.blue, '运行中'),
      RunStatus.pending => (Colors.orange, '等待中'),
      RunStatus.cancelled => (Colors.grey, '已取消'),
      RunStatus.timedOut => (Colors.red[700]!, '超时'),
      RunStatus.paused => (Colors.amber, '已暂停'),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              color: color,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 5),
          Text(label,
              style: TextStyle(color: color, fontSize: 12, fontWeight: FontWeight.w500)),
        ],
      ),
    );
  }
}

class NodeStatusBadge extends StatelessWidget {
  final NodeStatus status;

  const NodeStatusBadge({super.key, required this.status});

  @override
  Widget build(BuildContext context) {
    final (Color color, String label) = switch (status) {
      NodeStatus.succeeded => (Colors.green, '成功'),
      NodeStatus.failed => (Colors.red, '失败'),
      NodeStatus.running => (Colors.blue, '运行中'),
      NodeStatus.pending => (Colors.orange, '等待中'),
      NodeStatus.cancelled => (Colors.grey, '已取消'),
      NodeStatus.paused => (Colors.amber, '已暂停'),
      NodeStatus.waitingApproval => (Colors.purple, '等待审批'),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(label,
          style: TextStyle(color: color, fontSize: 11, fontWeight: FontWeight.w500)),
    );
  }
}