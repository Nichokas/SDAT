package auth

import (
	"net/http"
)

type AuthStrategy interface {
	Authenticate(req *http.Request, clientId string, clientSecret string) error
}
