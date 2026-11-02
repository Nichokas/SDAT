package tonearm

type Cover interface {
	// Animated returns the path to an animated cover image such as a GIF or video.
	Animated() string

	// HasAnimated returns true if the cover has an animated image.
	HasAnimated() bool

	// HasStatic returns true if the cover has a static image.
	HasStatic() bool

	// Static returns the path to a static cover image such as a PNG or JPEG.
	Static() string
}

// URLCover is a cover that is represented by a URL to a static and/or animated cover.
type URLCover struct {
	animatedURL string
	staticURL   string
}

func (c URLCover) Animated() string {
	return c.animatedURL
}

func (c URLCover) HasAnimated() bool {
	return c.animatedURL != ""
}

func (c URLCover) HasStatic() bool {
	return c.staticURL != ""
}

func (c URLCover) Static() string {
	return c.staticURL
}

func NewURLCover(animatedURL, staticURL string) URLCover {
	return URLCover{
		animatedURL: animatedURL,
		staticURL:   staticURL,
	}
}
