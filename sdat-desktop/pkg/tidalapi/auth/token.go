package auth

import (
	"encoding/json"
	"net/http"
	"net/url"
	"strings"

	"codeberg.org/dergs/tonearm/internal/settings"
)

type TokenResponse struct {
	AccessToken  string `json:"access_token"`
	ClientName   string `json:"clientName"`
	ExpiresIn    int    `json:"expires_in"`
	RefreshToken string `json:"refresh_token"`
	Scope        string `json:"scope"`
	TokenType    string `json:"token_type"`
	User         struct {
		ID    int    `json:"id"`
		Email string `json:"email"`
	} `json:"user"`
	UserID int `json:"user_id"`
}

type TokenError struct {
	Status           int    `json:"status"`
	Error            string `json:"error"`
	SubStatus        int    `json:"sub_status"`
	ErrorDescription string `json:"error_description"`
}

func Token(formValues url.Values) (*TokenResponse, *TokenError, error) {
	req, err := http.NewRequest(http.MethodPost, settings.ServiceTidal().AuthBaseURL()+"/v1/oauth2/token", strings.NewReader(formValues.Encode()))
	if err != nil {
		return nil, nil, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		var tokenError TokenError
		if err := json.NewDecoder(resp.Body).Decode(&tokenError); err != nil {
			return nil, nil, err
		}
		return nil, &tokenError, nil
	}
	var tokenResponse TokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&tokenResponse); err != nil {
		return nil, nil, err
	}
	return &tokenResponse, nil, nil
}
