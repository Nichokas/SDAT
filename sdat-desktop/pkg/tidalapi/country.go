package tidalapi

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
)

type countryCodeResponse struct {
	CountryCode string `json:"countryCode"`
}

func FetchCountryCode() (string, error) {
	req, err := http.NewRequestWithContext(context.TODO(), http.MethodGet, settings.ServiceTidal().APIBaseURL()+"/v1/country/context?countryCode=WW&locale=en_US&deviceType=BROWSER", nil)
	if err != nil {
		return "", err
	}

	req.Header.Set("x-tidal-token", internal.ClientID)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var response countryCodeResponse
	if err := json.NewDecoder(resp.Body).Decode(&response); err != nil {
		return "", err
	}

	return response.CountryCode, nil
}
