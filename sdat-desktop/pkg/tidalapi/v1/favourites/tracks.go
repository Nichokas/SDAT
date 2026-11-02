package favourites

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
)

type FavouriteTrack struct {
	client *internal.Client
}

func NewFavouriteTrack(client *internal.Client) *FavouriteTrack {
	return &FavouriteTrack{
		client: client,
	}
}

func (f *FavouriteTrack) Add(ctx context.Context, userID, trackId string) error {
	body := url.Values{
		"trackIds":           {trackId},
		"onArtifactNotFound": {"FAIL"},
	}
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPost,
		fmt.Sprintf("/v1/users/%s/favorites/tracks", userID),
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
		return fmt.Errorf("error while adding track to favourites: %s", errorText)
	}

	return nil
}

func (f *FavouriteTrack) Remove(ctx context.Context, userID, trackId string) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, fmt.Sprintf("/v1/users/%s/favorites/tracks/%s", userID, trackId), nil)
	if err != nil {
		return err
	}

	resp, err := f.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return err
	}

	return nil
}
