package track_manifests

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
)

func (t *TrackManifests) TrackManifest(ctx context.Context, id string, manifestType openapi.ManifestType, formats []openapi.ManifestFormat, uriScheme openapi.ManifestURIScheme, usage openapi.ManifestUsage, adaptive bool) (*openapi.TrackManifest, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/v2/trackManifests/%s", settings.ServiceTidal().OpenAPIBaseURL(), id), nil)
	if err != nil {
		return nil, err
	}

	params := req.URL.Query()
	params.Set("manifestType", string(manifestType))
	formatStrs := make([]string, len(formats))
	for i, f := range formats {
		formatStrs[i] = string(f)
	}
	params.Set("formats", strings.Join(formatStrs, ","))
	params.Set("uriScheme", string(uriScheme))
	params.Set("usage", string(usage))
	params.Set("adaptive", strconv.FormatBool(adaptive))
	req.URL.RawQuery = params.Encode()

	resp, err := t.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var manifest openapi.TrackManifest
	if err := json.NewDecoder(resp.Body).Decode(&manifest); err != nil {
		return nil, err
	}

	return &manifest, nil
}
