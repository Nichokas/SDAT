package tracks

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
)

func (t *Tracks) Mix(ctx context.Context, trackId int) (*v1.TrackMix, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/v1/tracks/"+fmt.Sprintf("%d", trackId)+"/mix", nil)
	if err != nil {
		return nil, err
	}

	resp, err := t.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var track v1.TrackMix
	if err := json.NewDecoder(resp.Body).Decode(&track); err != nil {
		return nil, err
	}

	return &track, nil
}
