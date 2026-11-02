package artist

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
)

func (a *Artist) Artist(ctx context.Context, artistId string) (*v2.ArtistPage, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("/v2/artist/%s", artistId), nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := a.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to execute request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var static v2.ArtistPage
	if err := json.NewDecoder(resp.Body).Decode(&static); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &static, nil
}
