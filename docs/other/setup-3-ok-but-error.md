commit `fc45815f42c41d93a24f7b0c42721352e2614ee8` でエラー修正してもらった結果, Step 4まで進みました.

でも何かのエラーが出て、それ以上先へ進めません。

何が原因か見当もつきません。調べてください。

```txt
🏗️  Huestatus Setup
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Welcome to huestatus! Let's configure your Philips Hue lights.

⚙️ Step 1/7: Initializing setup...

🔍 Step 2/7: Discovering Hue bridges...

🔑 Step 3/7: Authenticating with bridge at 192.168.1.146...

🔑 Authentication Required

To authenticate with your Hue bridge:
1. Press the large button on top of your Hue bridge
2. Wait for the button to start blinking
3. Press Enter to continue


Press the bridge button now and then press Enter...🔍 Attempting authentication...
⏳ Waiting for button press...
✅ Authentication successful!
🎉 You can now use huestatus!
💡 Step 4/7: Discovering lights...

💡Found 1 suitable light(s):
  ● テーブルランプ (1) - Color

🎨 Step 5/7: Creating status scenes...

❌ Configuration invalid: Invalid value: invalid value, null,, for parameter, effect. Run 'huestatus --setup' to fix.

💡 Try running:
   huestatus setup --force

For more help: https://github.com/mimikun/huestatus
```
