package artists

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
)

func (p *Artists) Artist(ctx context.Context, id string, include ...string) (*openapi.Artist, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/v2/artists/%s", settings.ServiceTidal().OpenAPIBaseURL(), id), nil)
	if err != nil {
		return nil, err
	}

	params := req.URL.Query()
	params.Set("include", strings.Join(include, ","))
	req.URL.RawQuery = params.Encode()

	resp, err := p.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var artist openapi.Artist
	if err := json.NewDecoder(resp.Body).Decode(&artist); err != nil {
		return nil, err
	}

	return &artist, nil
}
