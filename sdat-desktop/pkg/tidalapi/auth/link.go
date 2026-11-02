package auth

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/settings"
)

type DeviceLinkingChallenge struct {
	DeviceCode              string `json:"deviceCode"`
	UserCode                string `json:"userCode"`
	VerificationUri         string `json:"verificationUri"`
	VerificationUriComplete string `json:"verificationUriComplete"`
	ExpiresIn               int    `json:"expiresIn"`
	Interval                int    `json:"interval"`
}

func RequestDeviceLinkingChallenge(clientId string) (*DeviceLinkingChallenge, error) {
	formValues := url.Values{
		"client_id": []string{clientId},
		"scope":     []string{"r_usr w_usr w_sub"},
	}
	req, err := http.NewRequest(http.MethodPost, settings.ServiceTidal().AuthBaseURL()+"/v1/oauth2/device_authorization", strings.NewReader(formValues.Encode()))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("failed to get token: %s", resp.Status)
	}
	var tokenResponse DeviceLinkingChallenge
	if err := json.NewDecoder(resp.Body).Decode(&tokenResponse); err != nil {
		return nil, err
	}
	return &tokenResponse, nil
}

func PollDeviceLinkingStatus(ctx context.Context, clientId string, clientSecret string, deviceCode string, interval int) (*TokenResponse, error) {
	formValues := url.Values{
		"client_id":     []string{clientId},
		"client_secret": []string{clientSecret},
		"device_code":   []string{deviceCode},
		"grant_type":    []string{"urn:ietf:params:oauth:grant-type:device_code"},
		"scope":         []string{"r_usr w_usr w_sub"},
	}
	for {
		select {
		case <-ctx.Done():
			return nil, errors.New("linking canceled")
		default:
			tokenResponse, tokenError, err := Token(formValues)
			if err != nil {
				return nil, err
			}
			if tokenError != nil {
				if tokenError.Error == "authorization_pending" {
					time.Sleep(time.Duration(interval) * time.Second)
					continue
				}
				if tokenError.Error == "expired_token" {
					return nil, errors.New("linking session expired")
				}
				return nil, errors.New(tokenError.ErrorDescription)
			}
			return tokenResponse, nil
		}
	}
}
