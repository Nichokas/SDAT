package openapi

import (
	"encoding/json"
	"math"
	"sort"
)

type Artwork Response[ArtworkData]

const ObjectTypeArtworks = "artworks"

type ArtworkMediaType string

const (
	ArtworkMediaTypeImage ArtworkMediaType = "IMAGE"
	ArtworkMediaTypeVideo ArtworkMediaType = "VIDEO"
)

type ArtworkData struct {
	Attributes   ArtworkAttributes    `json:"attributes"`
	ID           string               `json:"id"`
	Relationship ArtworkRelationships `json:"relationships"`
	Type         string               `json:"type"`
}

type ArtworkAttributes struct {
	Files     ArtworkFiles     `json:"files"`
	MediaType ArtworkMediaType `json:"mediaType"`
}

func (a ArtworkAttributes) IsPicture() bool {
	return a.MediaType == ArtworkMediaTypeImage
}

type ArtworkRelationships struct {
	Owners Response[[]Relationship] `json:"owners"`
}

type ArtworkFile struct {
	Href string `json:"href"`
	Meta struct {
		Height int `json:"height"`
		Width  int `json:"width"`
	} `json:"meta"`
}

type ArtworkFiles []ArtworkFile

func (files ArtworkFiles) AtLeast(size int) *ArtworkFile {
	if len(files) == 0 {
		return nil
	}

	squareFiles := make(ArtworkFiles, 0, len(files))
	for _, file := range files {
		if file.Meta.Height == file.Meta.Width {
			squareFiles = append(squareFiles, file)
		}
	}
	if len(squareFiles) == 0 {
		squareFiles = files
	}

	sort.Slice(squareFiles, func(i, j int) bool {
		return squareFiles[i].Meta.Height < squareFiles[j].Meta.Height || squareFiles[i].Meta.Width < squareFiles[j].Meta.Width
	})

	if size == math.MaxInt {
		var (
			maxHeight int
			f         ArtworkFile
		)

		for _, file := range squareFiles {
			if file.Meta.Height > maxHeight {
				maxHeight = file.Meta.Height
				f = file
			}
		}

		return &f
	}

	for _, file := range squareFiles {
		if min(file.Meta.Height, file.Meta.Width) >= size {
			return &file
		}
	}
	return &squareFiles[0]
}

type Artworks []ArtworkData

func (artworks Artworks) AtLeast(size int, mediaType ArtworkMediaType) string {
	coverUrl := ""
	for _, artwork := range artworks {
		if artwork.Attributes.MediaType == mediaType {
			cover := artwork.Attributes.Files.AtLeast(size)
			if cover != nil {
				coverUrl = cover.Href
				break
			}
		}
	}
	return coverUrl
}

func (i IncludedObjects) PlainArtworks(relationships ...Relationship) Artworks {
	var objects = i.FromRelationships(relationships, ObjectTypeArtworks)

	var artworks []ArtworkData
	for _, obj := range objects {
		var artwork ArtworkData
		if err := json.Unmarshal(obj.Raw, &artwork); err != nil {
			continue
		}
		artworks = append(artworks, artwork)
	}
	return artworks
}
