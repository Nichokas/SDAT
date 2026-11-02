package auth

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/settings"
)

type OpenAPIClientAuthStrategy struct {
	cachedToken string
	tokenExpiry time.Time
}

func NewOpenAPIClientAuthStrategy() *OpenAPIClientAuthStrategy {
	return &OpenAPIClientAuthStrategy{}
}

func (s *OpenAPIClientAuthStrategy) GetToken(clientID, clientSecret string) (string, error) {
	if s.cachedToken != "" && time.Now().Before(s.tokenExpiry) {
		return s.cachedToken, nil
	}

	formValues := url.Values{
		"client_id":     []string{clientID},
		"client_secret": []string{clientSecret},
		"grant_type":    []string{"client_credentials"},
	}
	req, err := http.NewRequest(http.MethodPost, settings.ServiceTidal().AuthBaseURL()+"/v1/oauth2/token", strings.NewReader(formValues.Encode()))
	if err != nil {
		return "", err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("failed to get token: %s", resp.Status)
	}

	var tokenResponse struct {
		AccessToken string `json:"access_token"`
		ExpiresIn   int    `json:"expires_in"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&tokenResponse); err != nil {
		return "", err
	}

	s.cachedToken = tokenResponse.AccessToken
	s.tokenExpiry = time.Now().Add(time.Duration(tokenResponse.ExpiresIn) * time.Second)

	return tokenResponse.AccessToken, nil
}

func (s *OpenAPIClientAuthStrategy) Authenticate(req *http.Request, clientID, clientSecret string) error {
	// This auth is only valid for openapi.tidal.com
	if strings.Contains(req.URL.String(), settings.ServiceTidal().OpenAPIBaseURL()) && req.Header.Get("Authorization") == "" {
		token, err := s.GetToken(clientID, clientSecret)
		if err != nil {
			return err
		}

		req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", token))
	}

	return nil
}
