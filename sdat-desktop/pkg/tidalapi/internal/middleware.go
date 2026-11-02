package internal

import (
	"fmt"
	"net/http"
	"net/url"
	"time"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/auth"
	"github.com/jeandeaual/go-locale"
)

type MiddlewareRoundTripper struct {
	authStrategies []auth.AuthStrategy
	countryCode    string
}

func (m MiddlewareRoundTripper) RoundTrip(req *http.Request) (*http.Response, error) {
	// Configure the TIDAL Client version we currently replicate
	req.Header.Set("x-tidal-client-version", ClientVersion)
	req.Header.Set("x-tidal-token", UnauthenticatedClientID)

	// Identify ourselves to TIDAL, we want to be a friendly citizen on their API.
	req.Header.Set("User-Agent", "Tonearm/"+ClientVersion+" Mozilla/5.0 (Linux x86_64; rv:146.0) Gecko/20100101 Firefox/146.0")

	// Configure the base URL unless set
	baseUrl := settings.ServiceTidal().APIBaseURL()
	if req.URL.Scheme == "" || req.URL.Host == "" {
		req.URL.Scheme = ""
		req.URL.Host = ""
		parsed, err := url.Parse(baseUrl + req.URL.String())
		if err == nil {
			req.URL = parsed
		}
	}

	// Configure default query params
	queryParams := req.URL.Query()
	queryParams.Set("deviceType", "BROWSER")
	queryParams.Set("platform", "WEB")
	queryParams.Set("countryCode", m.countryCode)
	queryParams.Set("timeOffset", utcOffset(time.Now(), nil))

	// Detect locale based on system, fallback to en_US
	if userLocale, err := locale.GetLocale(); err == nil {
		queryParams.Set("locale", userLocale)
	} else {
		queryParams.Set("locale", "en_US")
	}

	req.URL.RawQuery = queryParams.Encode()

	// Execute all auth strategies
	for _, strategy := range m.authStrategies {
		if err := strategy.Authenticate(req, ClientID, ClientSecret); err != nil {
			return nil, err
		}
	}

	if req.Header.Get("authorization") != "" {
		req.Header.Set("x-tidal-token", ClientID)
	}

	// Hand off to the default transport
	return http.DefaultTransport.RoundTrip(req)
}

func utcOffset(t time.Time, loc *time.Location) string {
	if loc != nil {
		t = t.In(loc) // convert the instant to the target zone
	}
	_, offsetSec := t.Zone() // offset in seconds east of UTC

	sign := '+'
	if offsetSec < 0 {
		sign = '-'
		offsetSec = -offsetSec // make it positive for the math below
	}
	hours := offsetSec / 3600
	minutes := (offsetSec % 3600) / 60

	// fmt.Sprintf guarantees leading zeros (02 for width 2, zero‑pad)
	return fmt.Sprintf("%c%02d:%02d", sign, hours, minutes)
}
