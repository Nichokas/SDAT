package albums

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
)

func (p *Albums) Items(ctx context.Context, id string, cursor string, include ...string) (*openapi.Response[[]openapi.Relationship], error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/v2/albums/%s/relationships/items", settings.ServiceTidal().OpenAPIBaseURL(), id), nil)
	if err != nil {
		return nil, err
	}

	params := req.URL.Query()
	if cursor != "" {
		params.Set("page[cursor]", cursor)
	}
	if include != nil {
		params.Set("include", strings.Join(include, ","))
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

	var response openapi.Response[[]openapi.Relationship]
	if err := json.NewDecoder(resp.Body).Decode(&response); err != nil {
		return nil, err
	}

	return &response, nil
}
