package internal

import "encoding/base64"

const (
	clientIDPart1     = "WmxneVNuaGtiVzUw"
	clientIDPart2     = "V2xkTE1HbDRWQT09"
	clientSecretPart1 = "TVU1dU9VRm1SRUZxZUhKblNrWktZa3RPVjB4bFFY"
	clientSecretPart2 = "bExSMVpIYlVsT2RWaFFVRXhJVmxoQmRuaEJaejA9"

	ClientVersion           = "2025.12.13"
	UnauthenticatedClientID = "txNoH4kkV41MfH25"
)

var (
	ClientID     string
	ClientSecret string
)

func init() {
	clientIdPart1Decoded, _ := base64.StdEncoding.DecodeString(clientIDPart1)
	clientIdPart2Decoded, _ := base64.StdEncoding.DecodeString(clientIDPart2)
	clientIdDecoded, _ := base64.StdEncoding.DecodeString(string(clientIdPart1Decoded) + string(clientIdPart2Decoded))
	ClientID = string(clientIdDecoded)

	clientSecretPart1Decoded, _ := base64.StdEncoding.DecodeString(clientSecretPart1)
	clientSecretPart2Decoded, _ := base64.StdEncoding.DecodeString(clientSecretPart2)
	clientSecretDecoded, _ := base64.StdEncoding.DecodeString(string(clientSecretPart1Decoded) + string(clientSecretPart2Decoded))
	ClientSecret = string(clientSecretDecoded)
}
