import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Android Keystore backed secret storage.
/// API Keys are stored in Android Keystore via flutter_secure_storage.
/// The frontend CANNOT read back stored secret values — they are write-only
/// from the app's perspective (Keychain/Keystore encrypted storage).
class SecretService {
  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );

  /// Store a secret (API key) in Android Keystore.
  /// Key format: provider/<provider_id>
  static Future<void> storeSecret(String key, String value) async {
    await _storage.write(key: key, value: value);
  }

  /// Read a secret from secure storage.
  /// Only the Rust Core background worker should call this.
  static Future<String?> readSecret(String key) async {
    return _storage.read(key: key);
  }

  /// Delete a secret.
  static Future<void> deleteSecret(String key) async {
    await _storage.delete(key: key);
  }

  /// Check if a secret exists.
  static Future<bool> secretExists(String key) async {
    final value = await _storage.read(key: key);
    return value != null;
  }

  /// List all stored secret keys (never returns values).
  static Future<List<String>> listSecretKeys() async {
    final all = await _storage.readAll();
    return all.keys.toList();
  }

  /// Export-safe: returns which providers have keys, never the keys themselves.
  static Future<Map<String, bool>> getSecretStatus(List<String> providerIds) async {
    final result = <String, bool>{};
    for (final id in providerIds) {
      result[id] = await secretExists('provider/$id');
    }
    return result;
  }
}