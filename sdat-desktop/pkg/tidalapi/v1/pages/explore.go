package pages

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
)

func (p *Pages) Page(ctx context.Context, path string) (*v1.Page, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/v1/pages/"+path, nil)
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

	var page v1.Page
	if err := json.NewDecoder(resp.Body).Decode(&page); err != nil {
		return nil, err
	}

	return &page, nil
}
