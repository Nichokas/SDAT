package favourites

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
)

type FavouriteMixes struct {
	client *internal.Client
}

func NewFavouriteMixes(client *internal.Client) *FavouriteMixes {
	return &FavouriteMixes{
		client: client,
	}
}

func (f *FavouriteMixes) Add(ctx context.Context, mixUUID string) error {
	body := url.Values{
		"mixIds":             {mixUUID},
		"onArtifactNotFound": {"FAIL"},
	}
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPut,
		"/v2/favorites/mixes/add",
		strings.NewReader(body.Encode()),
	)
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	if err != nil {
		return err
	}

	resp, err := f.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		errorText, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("error while adding mix to favourites: %s", errorText)
	}

	return nil
}

func (f *FavouriteMixes) IDs(ctx context.Context, cursor, ifNoneMatch string) (data *v2.FavouritesIds, etag string, err error) {
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodGet,
		"/v2/favorites/mixes/ids",
		nil,
	)
	if err != nil {
		return nil, "", err
	}

	params := url.Values{}
	params.Add("limit", "50")
	if cursor != "" {
		params.Add("cursor", cursor)
	}
	req.URL.RawQuery = params.Encode()

	if ifNoneMatch != "" {
		req.Header.Set("If-None-Match", ifNoneMatch)
	}

	resp, err := f.client.Do(req)
	if err != nil {
		return nil, "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		errorText, _ := io.ReadAll(resp.Body)
		return nil, "", fmt.Errorf("error while retrieving favourite mix IDs: %s", errorText)
	}

	data = &v2.FavouritesIds{}
	if err := json.NewDecoder(resp.Body).Decode(data); err != nil {
		return nil, "", err
	}

	return data, resp.Header.Get("ETag"), nil
}

func (f *FavouriteMixes) Remove(ctx context.Context, mixUUID string) error {
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPut,
		"/v2/favorites/mixes/remove",
		nil)
	if err != nil {
		return err
	}

	params := url.Values{}
	params.Add("mixIds", mixUUID)
	req.URL.RawQuery = params.Encode()

	resp, err := f.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("error while removing mix from favourites: %s", resp.Status)
	}

	return nil
}
