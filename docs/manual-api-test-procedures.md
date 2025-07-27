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
    "swupdate": Object{
      "state": String,
      "lastinstall": String (ISO 8601)
    },
    "type": String,
    "name": String,              // 日本語名もサポート
    "modelid": String,
    "manufacturername": String,
    "productname": String,
    "capabilities": Object{...},
    "config": Object{
      "archetype": String,
      "function": String,
      "direction": String,
      "startup": Object{         // 追加フィールド
        "mode": String,
        "configured": Boolean
      }
    },
    "uniqueid": String,          // 新発見フィールド
    "swversion": String,
    "swconfigid": String,        // 新発見フィールド
    "productid": String          // 新発見フィールド
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

### 実証済み知見（2025-07-27テスト）
- **日本語ライト名**: UTF-8で正常にサポートされる
- **追加フィールド**: APIドキュメントに記載のない追加フィールドが存在
  - `swupdate`: ソフトウェアアップデート情報
  - `uniqueid`: デバイス固有ID
  - `swconfigid`, `productid`: 製品管理ID
  - `config.startup`: 起動時設定
- **カラーガマット**: 実際のデバイスはタイプ"C"（理論値"B"と異なる）
- **デバイス多様性**: Signe gradient table等の特殊デバイスも正常動作

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

## 🧪 実際のテスト結果 (2025-07-27実行)

### テスト実行日時
2025-07-27 19:00:00

### Phase 1結果: 基本接続テスト ✅
```json
[{"error":{"type":4,"address":"/","description":"method, GET, not available for resource, /"}}]
```
**結果**: 期待通りのエラーレスポンス。Bridgeとの接続は正常。

### Phase 2結果: 認証フローテスト ✅

#### ボタン押下前
```json
[
  {
    "error": {
      "type": 101,
      "address": "",
      "description": "link button not pressed"
    }
  }
]
```

#### ボタン押下後（認証成功）
```json
[{"success":{"username":"tcQ3Sv9KwZnCvNApXKNYJdFNBTPTEn4fGPdjhuiZ"}}]
```
**結果**: 40文字のusernameが正常に取得できた。

### Phase 3結果: 認証後APIテスト ✅

#### ライト一覧取得
```json
{
  "1": {
    "state": {
      "on": false,
      "bri": 254,
      "hue": 8401,
      "sat": 142,
      "effect": "none",
      "xy": [0.459, 0.4103],
      "ct": 369,
      "alert": "select",
      "colormode": "ct",
      "mode": "homeautomation",
      "reachable": true
    },
    "swupdate": { "state": "noupdates", "lastinstall": "2025-07-16T18:17:54" },
    "type": "Extended color light",
    "name": "テーブルランプ",
    "modelid": "929003555601",
    "manufacturername": "Signify Netherlands B.V.",
    "productname": "Signe gradient table",
    "capabilities": {
      "certified": true,
      "control": {
        "mindimlevel": 10,
        "maxlumen": 700,
        "colorgamuttype": "C",
        "colorgamut": [
          [0.6915, 0.3083],
          [0.17, 0.7],
          [0.1532, 0.0475]
        ],
        "ct": { "min": 153, "max": 500 }
      },
      "streaming": { "renderer": true, "proxy": true }
    },
    "config": {
      "archetype": "huesigne",
      "function": "decorative",
      "direction": "horizontal",
      "startup": { "mode": "safety", "configured": true }
    },
    "uniqueid": "00:17:88:01:0c:53:de:b4-0b",
    "swversion": "1.122.8",
    "swconfigid": "2E841ADB",
    "productid": "4422-9482-0441_HG01_PSU22"
  }
}
```

#### シーン作成テスト
```json
[{ "success": { "id": "5By1Sk30AxAeffr" } }]
```

### 分析結果 ✅
- **エラーレスポンス構造の確認**: ✅ 期待通りの構造
- **成功レスポンス構造の確認**: ✅ 配列形式でsuccess/errorキーを含む
- **コードとの整合性**: ✅ Rustコードで想定している構造と一致
- **発見した新事項**:
  - 日本語ライト名「テーブルランプ」をサポート
  - Signe gradient tableモデル（予想と異なるデバイス）
  - 追加フィールド: `swupdate`, `uniqueid`, `swconfigid`, `productid`
  - `config.startup`フィールドの存在
  - 実際のcolorgamuttypeは"C"（理論値"B"と異なる）

---

## 📚 参考資料

- [Philips Hue API Documentation](https://developers.meethue.com/develop/get-started-2/)
- [Hue API Authentication](https://developers.meethue.com/develop/application-design-guidance/hue-bridge-discovery/)
- [プロジェクト内認証コード](../src/bridge/auth.rs)