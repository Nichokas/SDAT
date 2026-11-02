package openapi

type UserData struct {
	Attributes UserAttributes `json:"attributes"`
	ID         string         `json:"id"`
	Type       string         `json:"type"`
}

type UserAttributes struct {
	Country       string `json:"country"`
	Email         string `json:"email"`
	EmailVerified bool   `json:"emailVerified"`
	FirstName     string `json:"firstName"`
	LastName      string `json:"lastName"`
	Username      string `json:"username"`
}
