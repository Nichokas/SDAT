package favourites

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
)

func (f *Favourites) IDsWithCache(ctx context.Context, userID string, ifNoneMatch string) (list *v1.FavouritesIdLists, etag string, notModified bool, err error) {
	var req *http.Request
	req, err = http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("/v1/users/%s/favorites/ids", userID), nil)
	if err != nil {
		return
	}
	if ifNoneMatch != "" {
		req.Header.Add("If-None-Match", ifNoneMatch)
	}

	var resp *http.Response
	resp, err = f.client.Do(req)
	if err != nil {
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode == http.StatusNotModified {
		notModified = true
		return
	}

	if resp.StatusCode != http.StatusOK {
		err = fmt.Errorf("unexpected status code: %d", resp.StatusCode)
		return
	}

	if err = json.NewDecoder(resp.Body).Decode(&list); err != nil {
		return
	}

	etag = resp.Header.Get("etag")

	return
}

func (f *Favourites) IDs(ctx context.Context, userID string) (*v1.FavouritesIdLists, error) {
	list, _, _, err := f.IDsWithCache(ctx, userID, "")
	if err != nil {
		return nil, err
	}
	return list, nil
}
