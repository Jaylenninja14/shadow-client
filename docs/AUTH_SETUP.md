# Microsoft sign-in setup

Shadow Client never asks for your Microsoft password. It uses Microsoft's **OAuth 2.0 device-code flow**: the launcher shows a short code, you sign in on microsoft.com, and the launcher receives tokens for that sign-in only. Tokens are encrypted with Windows DPAPI (bound to your Windows user) and refreshed automatically.

## Why a setup step is needed

Microsoft only issues Minecraft tokens to **applications registered in Azure and approved for the Minecraft API**. Every third-party launcher needs its own application (client) ID. Using another launcher's ID would be impersonation, so Shadow Client ships **without** one. Until you add an approved ID:

* Offline profiles work for singleplayer/LAN testing (clearly labelled).
* The official Minecraft Launcher keeps working with your normal account (Home → account menu → *Open official launcher*). Shadow Client never reads or copies the official launcher's sign-in.

## 1. Register an Azure application

1. Go to <https://portal.azure.com> → **Microsoft Entra ID** → **App registrations** → **New registration**.
2. Name: anything (e.g. “Shadow Client – <your name>”).
3. Supported account types: **Personal Microsoft accounts only** (or “Accounts in any organizational directory and personal Microsoft accounts”).
4. Redirect URI: leave empty (device-code flow does not use one).
5. After creating it, open **Authentication** → enable **Allow public client flows** → Save.
6. Copy the **Application (client) ID** (a GUID).

No client secret is needed or used.

## 2. Request Minecraft API access

New Azure apps receive HTTP 403 from `api.minecraftservices.com` until Mojang approves them. Submit the app ID through Mojang's review form (linked from <https://aka.ms/mce-reviewappid>) and wait for approval. Shadow Client shows a clear message (“Minecraft services rejected this app's sign-in…”) while the ID is not yet approved.

## 3. Add the ID to Shadow Client

Settings → **Accounts & services** → *Azure application (client) ID* → paste → **Save**.
Then open the account menu (top right) → **Sign in with Microsoft**, open the shown page, enter the code.

## What happens during sign-in

| Step | Endpoint | Purpose |
|---|---|---|
| Device code | `login.microsoftonline.com/consumers/oauth2/v2.0/devicecode` | get the code you type on microsoft.com |
| Token | `…/oauth2/v2.0/token` | Microsoft access + refresh token (scope `XboxLive.signin offline_access`) |
| Xbox Live | `user.auth.xboxlive.com` | Xbox user token |
| XSTS | `xsts.auth.xboxlive.com` | token for Minecraft services; clear errors for no Xbox profile, child accounts, region blocks |
| Login | `api.minecraftservices.com/authentication/login_with_xbox` | Minecraft access token |
| Ownership | `…/entitlements/mcstore`, `…/minecraft/profile` | confirms the account owns Java Edition and has a profile name |

Only the refresh token and the current Minecraft token (with its expiry) are stored, encrypted. Sign out by removing the account in the account dialog.
