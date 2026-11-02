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

type FavouriteArtist struct {
	client *internal.Client
}

func NewFavouriteArtist(client *internal.Client) *FavouriteArtist {
	return &FavouriteArtist{
		client: client,
	}
}

func (f *FavouriteArtist) Add(ctx context.Context, userID, artistId string) error {
	body := url.Values{
		"artistIds":          {artistId},
		"onArtifactNotFound": {"FAIL"},
	}
	req, err := http.NewRequestWithContext(
		ctx,
		http.MethodPost,
		fmt.Sprintf("/v1/users/%s/favorites/artists", userID),
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
		return fmt.Errorf("error while adding artist to favourites: %s", errorText)
	}

	return nil
}

func (f *FavouriteArtist) Remove(ctx context.Context, userID, artistId string) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, fmt.Sprintf("/v1/users/%s/favorites/artists/%s", userID, artistId), nil)
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
