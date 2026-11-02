package playlists

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
)

func (p *Playlists) Playlist(ctx context.Context, uuid string, include ...string) (*openapi.Playlist, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/v2/playlists/%s", settings.ServiceTidal().OpenAPIBaseURL(), uuid), nil)
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

	var playlist openapi.Playlist
	if err := json.NewDecoder(resp.Body).Decode(&playlist); err != nil {
		return nil, err
	}

	return &playlist, nil
}
