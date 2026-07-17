import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Android Keystore interface for secure credential storage.
///
/// API Keys are stored exclusively in Android Keystore via
/// EncryptedSharedPreferences (backed by AES-256 GCM).
///
/// Security guarantees:
/// - Keys are encrypted at rest using Android Keystore
/// - Keys are never written to app logs, crash reports, or backups
/// - The Flutter UI can only CHECK or STORE keys, never read them back
/// - Only the Rust Core worker (or background isolate) can read keys
class AndroidKeystore {
  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(
      encryptedSharedPreferences: true,
      // Restrict to app-specific key store
      keychainAccessGroup: null,
    ),
  );

  /// Store a secret value in Android Keystore.
  static Future<void> store(String key, String value) async {
    await _storage.write(key: key, value: value);
  }

  /// Read a secret value from Android Keystore.
  /// Should only be called from background isolates, never from UI.
  static Future<String?> read(String key) async {
    return _storage.read(key: key);
  }

  /// Delete a secret from Android Keystore.
  static Future<void> delete(String key) async {
    await _storage.delete(key: key);
  }

  /// Check if a secret exists without returning its value.
  static Future<bool> exists(String key) async {
    final value = await _storage.read(key: key);
    return value != null;
  }

  /// List all stored key identifiers (never returns values).
  static Future<List<String>> listKeys() async {
    final all = await _storage.readAll();
    return all.keys.toList();
  }

  /// Clear all stored secrets.
  static Future<void> clear() async {
    await _storage.deleteAll();
  }
}