class ProviderInfo {
  final String id;
  final String name;
  final String providerType;
  final String baseUrl;
  final bool hasApiKey;

  const ProviderInfo({
    required this.id,
    required this.name,
    required this.providerType,
    required this.baseUrl,
    this.hasApiKey = false,
  });

  factory ProviderInfo.fromJson(Map<String, dynamic> json) {
    return ProviderInfo(
      id: json['id'] as String,
      name: json['name'] as String,
      providerType: json['provider_type'] as String? ?? 'openai_compatible',
      baseUrl: json['base_url'] as String? ?? '',
      hasApiKey: json['has_api_key'] ?? false,
    );
  }
}

class SecretInfo {
  final String key;
  final bool exists;

  const SecretInfo({required this.key, required this.exists});

  factory SecretInfo.fromJson(Map<String, dynamic> json) {
    return SecretInfo(
      key: json['key'] as String,
      exists: json['exists'] ?? false,
    );
  }
}