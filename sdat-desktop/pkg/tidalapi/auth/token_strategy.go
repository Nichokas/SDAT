package auth

import (
	"fmt"
	"net/http"
)

type TokenAuthStrategy struct {
	token string
}

func NewTokenAuthStrategy(token string) *TokenAuthStrategy {
	return &TokenAuthStrategy{
		token: token,
	}
}

func (s *TokenAuthStrategy) Authenticate(req *http.Request, clientID, clientSecret string) error {
	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", s.token))
	return nil
}
