package playlists

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"

	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
)

type ItemsOptions struct {
	Limit  int `json:"limit"`
	Offset int `json:"offset"`
}

func (p *Playlists) Items(ctx context.Context, playlistUUID string, opts *ItemsOptions) (*v1.PlaylistItems, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/v1/playlists/"+playlistUUID+"/items", nil)
	if err != nil {
		return nil, err
	}

	params := url.Values{}
	if opts != nil {
		if opts.Limit > 0 {
			params.Set("limit", fmt.Sprintf("%d", opts.Limit))
		} else {
			params.Set("limit", "50")
		}
		if opts.Offset > 0 {
			params.Set("offset", fmt.Sprintf("%d", opts.Offset))
		}
	}
	req.URL.RawQuery = params.Encode()

	resp, err := p.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var playlistItems v1.PlaylistItems
	if err := json.NewDecoder(resp.Body).Decode(&playlistItems); err != nil {
		return nil, err
	}

	return &playlistItems, nil
}
