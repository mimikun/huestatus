# 🔧 Hue API 手動テスト手順

## 📋 目的

Hue Bridge API の認証フローを手動でテストし、実際のレスポンス構造を確認する。
これにより、コードの期待値と実際のAPIレスポンスの差異を特定する。

## 🏗️ 前提条件

- Hue Bridge IP: `192.168.1.146`
- ネットワーク接続が正常
- `curl`コマンドが利用可能
- Hue Bridgeの物理アクセスが可能

## 📝 テスト手順

### Phase 1: 基本接続テスト

#### 1.1 Bridge接続確認
```bash
curl -v http://192.168.1.146/api
```

**期待される結果:**
```json
[{"error":{"type":4,"address":"/","description":"method, GET, not available for resource, /"}}]
```

### Phase 2: 認証フローテスト

#### 2.1 ボタン押下前テスト（エラーレスポンス確認）
```bash
curl -X POST http://192.168.1.146/api \
-H "Content-Type: application/json" \
-d '{"devicetype":"huestatus#manual-test-before"}'
```

**期待される結果:**
```json
[{"error":{"type":101,"address":"","description":"link button not pressed"}}]
```

#### 2.2 ボタン押下後テスト（成功レスポンス確認）

**手順:**
1. **Hue Bridgeの物理ボタンを押す**（上部の大きな丸いボタン）
2. **ライトが点滅することを確認**（認証モード開始の合図）
3. **30秒以内に以下のコマンドを実行:**

```bash
curl -X POST http://192.168.1.146/api \
-H "Content-Type: application/json" \
-d '{"devicetype":"huestatus#manual-test-success"}'
```

**期待される結果（成功時）:**
```json
[{"success":{"username":"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"}}]
```

**期待される結果（失敗時）:**
```json
[{"error":{"type":101,"address":"","description":"link button not pressed"}}]
```

### Phase 3: 認証後APIテスト

#### 3.1 ライト一覧取得
成功時に取得したusernameを使用:

```bash
# {username}を実際の値に置き換える
curl http://192.168.1.146/api/{username}/lights
```

**期待される結果:**
```json
{
  "1": {
    "state": {
      "on": true,
      "bri": 254,
      "hue": 0,
      "sat": 0,
      "effect": "none",
      "xy": [0.3127, 0.3290],
      "ct": 153,
      "alert": "none",
      "colormode": "ct",
      "mode": "homeautomation",
      "reachable": true
    },
    "type": "Extended color light",
    "name": "Hue color lamp 1",
    "modelid": "LCT001",
    "manufacturername": "Philips",
    "productname": "Hue color lamp",
    "capabilities": {
      "certified": true,
      "control": {
        "mindimlevel": 5000,
        "maxlumen": 600,
        "colorgamuttype": "B",
        "colorgamut": [[0.675, 0.322], [0.409, 0.518], [0.167, 0.04]],
        "ct": {"min": 153, "max": 500}
      },
      "streaming": {
        "renderer": true,
        "proxy": false
      }
    },
    "config": {
      "archetype": "sultanbulb",
      "function": "mixed",
      "direction": "omnidirectional"
    },
    "swversion": "1.104.2"
  }
}
```

#### 3.2 シーン作成テスト
```bash
curl -X POST http://192.168.1.146/api/{username}/scenes \
-H "Content-Type: application/json" \
-d '{
  "name": "test-scene",
  "lights": ["1"],
  "recycle": true,
  "lightstates": {
    "1": {
      "on": true,
      "bri": 254,
      "hue": 21845,
      "sat": 254
    }
  }
}'
```

**期待される結果:**
```json
[{"success":{"id":"XXXXXXXXXXXX"}}]
```

## 📊 レスポンス構造分析

### 認証エラーレスポンス構造
```
Array[
  Object{
    "error": Object{
      "type": Number,
      "address": String,
      "description": String
    }
  }
]
```

### 認証成功レスポンス構造
```
Array[
  Object{
    "success": Object{
      "username": String(40文字)
    }
  }
]
```

### ライトデータ構造
```
Object{
  "{lightId}": Object{
    "state": Object{...},
    "type": String,
    "name": String,
    "modelid": String,
    "manufacturername": String,
    "productname": String,
    "capabilities": Object{...},
    "config": Object{...},
    "swversion": String
  }
}
```

## 🔍 重要なポイント

### 認証タイミング
- ボタン押下後、約30秒間のみ認証可能
- ライト点滅が認証モードの合図
- タイムアウト後は再度ボタン押下が必要

### エラーコード
- `type: 4` - メソッドが利用不可
- `type: 101` - リンクボタン未押下
- `type: 1` - 認証失敗

### レスポンス形式
- 全てのレスポンスが配列形式 `[{...}]`
- 成功時は `"success"` キー
- エラー時は `"error"` キー

## 🚨 トラブルシューティング

### 認証が成功しない場合
1. ボタンを確実に押す（クリック音が聞こえるまで）
2. ライトの点滅を確認
3. 30秒以内にコマンド実行
4. ネットワーク接続を確認

### Bridge接続ができない場合
1. IPアドレスの確認
2. ネットワーク接続の確認
3. Bridgeの電源確認
4. ファイアウォール設定の確認

## 📝 テスト結果記録フォーマット

```markdown
## テスト実行日時
YYYY-MM-DD HH:MM:SS

## Phase 1結果
### 基本接続テスト
```json
実際のレスポンス
```

## Phase 2結果
### ボタン押下前
```json
実際のレスポンス
```

### ボタン押下後
```json
実際のレスポンス
```

## Phase 3結果
### ライト一覧
```json
実際のレスポンス（一部省略可）
```

## 分析結果
- エラーレスポンス構造の確認: ✅/❌
- 成功レスポンス構造の確認: ✅/❌
- コードとの整合性: ✅/❌
- 発見した問題点: 記述
```

---

## 📚 参考資料

- [Philips Hue API Documentation](https://developers.meethue.com/develop/get-started-2/)
- [Hue API Authentication](https://developers.meethue.com/develop/application-design-guidance/hue-bridge-discovery/)
- [プロジェクト内認証コード](../src/bridge/auth.rs)