package secrets

import (
	"encoding/base64"
	"encoding/json"
	"log/slog"
	"strconv"
	"strings"
)

const (
	refreshTokenKey = "refresh_token"
)

func HasRefreshToken() bool {
	exists, err := getService().Has(refreshTokenKey)
	if err != nil {
		slog.Error("error checking refresh token in keyring", "error", err)
		return false
	}
	return exists
}

func GetRefreshToken() string {
	token, err := getService().Get(refreshTokenKey)
	if err != nil {
		slog.Error("error reading refresh token from keyring", "error", err)
		return ""
	}
	return token.Password
}

func SetRefreshToken(token string) error {
	defer triggerSignedInChanged()
	return getService().Set(refreshTokenKey, Item{Label: "Tonearm TIDAL Refresh Token", Password: token})
}

func DeleteRefreshToken() error {
	defer triggerSignedInChanged()
	err := getService().Delete(refreshTokenKey)
	if err != nil {
		slog.Error("error deleting refresh token from keyring", "error", err)
		return err
	}
	return nil
}

func UserID() string {
	if !HasRefreshToken() {
		return ""
	}

	token := GetRefreshToken()

	// Split JWT into parts (header.payload.signature)
	parts := strings.Split(token, ".")
	if len(parts) != 3 {
		return ""
	}

	// Decode the payload (second part)
	payload, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return ""
	}

	// Parse JSON payload
	var claims struct {
		UID int `json:"uid"`
	}
	if err := json.Unmarshal(payload, &claims); err != nil {
		return ""
	}

	return strconv.Itoa(claims.UID)
}
