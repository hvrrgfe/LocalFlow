import 'dart:convert';
import 'dart:io';
import 'package:http/http.dart' as http;

/// Handles external API calls (OpenAI compatible, custom HTTP).
/// All API keys come from SecretService — never from config or logs.
class ApiService {
  static const _defaultTimeout = Duration(seconds: 10);
  static const _modelTimeout = Duration(seconds: 120);

  /// Call an OpenAI-compatible chat completion API.
  /// [baseUrl] - API base URL (e.g. https://api.openai.com/v1)
  /// [apiKey] - API key from secret storage
  /// [model] - Model ID
  /// [messages] - Chat messages
  /// [temperature] - Sampling temperature
  /// [maxTokens] - Max tokens to generate
  /// [cancelToken] - For cancellation support
  static Future<Map<String, dynamic>> chatCompletion({
    required String baseUrl,
    required String apiKey,
    required String model,
    required List<Map<String, String>> messages,
    double? temperature,
    int? maxTokens,
    void Function()? cancelToken,
  }) async {
    final url = Uri.parse('${baseUrl.replaceAll(RegExp(r'/+$'), '')}/chat/completions');

    final body = <String, dynamic>{
      'model': model,
      'messages': messages,
    };
    if (temperature != null) body['temperature'] = temperature;
    if (maxTokens != null) body['max_tokens'] = maxTokens;

    final request = http.Request('POST', url)
      ..headers['Content-Type'] = 'application/json'
      ..headers['Authorization'] = 'Bearer $apiKey'
      ..body = jsonEncode(body);

    try {
      final streamed = await http.Client().send(request).timeout(_modelTimeout);
      final response = await http.Response.fromStream(streamed);

      if (response.statusCode == 200) {
        return jsonDecode(response.body) as Map<String, dynamic>;
      }

      // Safely extract error without exposing API key
      final errorBody = _safeErrorMessage(response.body);
      throw ApiException(response.statusCode, errorBody);
    } on SocketException {
      throw ApiException(0, '网络连接失败，请检查网络和代理设置');
    } on HttpException {
      throw ApiException(0, 'HTTP 请求错误');
    } on FormatException {
      throw ApiException(0, '无效的 API 响应格式');
    }
  }

  /// Execute a custom HTTP request.
  static Future<Map<String, dynamic>> customHttpRequest({
    required String url,
    required String method,
    Map<String, String>? headers,
    Map<String, dynamic>? body,
    String? apiKey,
    void Function()? cancelToken,
  }) async {
    final uri = Uri.parse(url);

    final allHeaders = <String, String>{
      'Content-Type': 'application/json',
      if (apiKey != null) 'Authorization': 'Bearer $apiKey',
      if (headers != null) ...headers,
    };

    http.Request request;
    switch (method.toUpperCase()) {
      case 'GET':
        request = http.Request('GET', uri);
      case 'POST':
        request = http.Request('POST', uri)
          ..body = body != null ? jsonEncode(body) : '';
      case 'PUT':
        request = http.Request('PUT', uri)
          ..body = body != null ? jsonEncode(body) : '';
      case 'DELETE':
        request = http.Request('DELETE', uri);
      default:
        request = http.Request('GET', uri);
    }

    request.headers.addAll(allHeaders);

    try {
      final streamed = await http.Client().send(request).timeout(_defaultTimeout);
      final response = await http.Response.fromStream(streamed);

      return {
        'status_code': response.statusCode,
        'body': _tryParseJson(response.body) ?? response.body,
        'headers': response.headers,
      };
    } on SocketException {
      throw ApiException(0, '网络连接失败');
    } on HttpException {
      throw ApiException(0, 'HTTP 请求错误');
    }
  }

  /// Sanitize error messages — strip any Authorization/Bearer content.
  static String _safeErrorMessage(String body) {
    try {
      final json = jsonDecode(body);
      if (json is Map && json['error'] != null) {
        final err = json['error'];
        if (err is Map && err['message'] != null) {
          return err['message'].toString().replaceAll(RegExp(r'(?i)(Bearer\s+)\S+'), '$1***');
        }
        if (err is String) return err;
      }
      return 'HTTP ${(json is Map && json['status_code'] != null) ? json['status_code'] : 'error'}';
    } catch (_) {
      return '请求失败 (HTTP error)';
    }
  }

  static dynamic _tryParseJson(String text) {
    try {
      return jsonDecode(text);
    } catch (_) {
      return null;
    }
  }
}

class ApiException implements Exception {
  final int statusCode;
  final String message;

  const ApiException(this.statusCode, this.message);

  @override
  String toString() => 'API 错误 ($statusCode): $message';
}