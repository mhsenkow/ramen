#!/usr/bin/env bash
# Submit a signed .app to Apple notarization and staple the ticket.
#
# Credentials (pick one):
#   A) Keychain profile (local):
#        NOTARY_KEYCHAIN_PROFILE=walstad-loom-notary ./scripts/notarize_macos_app.sh APP
#   B) App Store Connect API key (CI):
#        NOTARY_API_KEY=AuthKey_XXXX.p8 NOTARY_API_KEY_ID=... NOTARY_API_ISSUER=...
#   C) Inline app-specific password:
#        APPLE_ID=... APPLE_APP_SPECIFIC_PASSWORD=... APPLE_TEAM_ID=WC44W2QVE4
set -euo pipefail

APP="${1:-}"
ZIP="${2:-$(mktemp -t rama-cycle-notarize.XXXXXX.zip)}"
CLEANUP_ZIP=0
if [[ $# -lt 2 ]]; then
	CLEANUP_ZIP=1
fi

TEAM_ID="${APPLE_TEAM_ID:-WC44W2QVE4}"

if [[ -z "$APP" || ! -d "$APP" ]]; then
	echo "Usage: $0 /path/to/RAMA CYCLE.app [zip]" >&2
	exit 1
fi

echo "Creating notarization zip: $ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"

NOTARY_ARGS=(submit "$ZIP" --wait)
if [[ -n "${NOTARY_KEYCHAIN_PROFILE:-}" ]]; then
	NOTARY_ARGS+=(--keychain-profile "$NOTARY_KEYCHAIN_PROFILE")
elif [[ -n "${NOTARY_API_KEY:-}" && -n "${NOTARY_API_KEY_ID:-}" && -n "${NOTARY_API_ISSUER:-}" ]]; then
	KEY_FILE="$NOTARY_API_KEY"
	if [[ ! -f "$KEY_FILE" ]]; then
		# CI often stores the .p8 contents as the secret value.
		KEY_FILE="$(mktemp -t AuthKey.XXXXXX.p8)"
		printf '%s' "$NOTARY_API_KEY" > "$KEY_FILE"
	fi
	NOTARY_ARGS+=(--key "$KEY_FILE" --key-id "$NOTARY_API_KEY_ID" --issuer "$NOTARY_API_ISSUER")
elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" ]]; then
	NOTARY_ARGS+=(--apple-id "$APPLE_ID" --password "$APPLE_APP_SPECIFIC_PASSWORD" --team-id "$TEAM_ID")
else
	echo "No notarization credentials. Set NOTARY_KEYCHAIN_PROFILE or API key or APPLE_ID+password." >&2
	exit 1
fi

echo "Submitting to Apple notarization (this can take several minutes)..."
xcrun notarytool "${NOTARY_ARGS[@]}"

echo "Stapling ticket to app..."
xcrun stapler staple "$APP"
xcrun stapler validate "$APP"

if [[ "$CLEANUP_ZIP" == "1" ]]; then
	rm -f "$ZIP"
fi

echo "Notarized OK: $APP"
