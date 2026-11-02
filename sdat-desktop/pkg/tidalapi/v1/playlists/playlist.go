package playlists

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
)

func (p *Playlists) Playlist(ctx context.Context, playlistUUID string) (*v1.Playlist, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/v1/playlists/"+playlistUUID, nil)
	if err != nil {
		return nil, err
	}

	resp, err := p.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var playlist v1.Playlist
	if err := json.NewDecoder(resp.Body).Decode(&playlist); err != nil {
		return nil, err
	}

	return &playlist, nil
}
