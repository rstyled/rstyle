// Regression for go-git parseAt: a branch deliberately parses the next token's literal.
package fixture

import "strconv"

type Parser struct{}

func (p *Parser) scan() (int, string)      { return 1, "1" }
func (p *Parser) parseOne() (int, error)   { _, lit := p.scan(); return strconv.Atoi(lit) }
func (p *Parser) parseTwo() (int, error)   { _, lit := p.scan(); return strconv.Atoi(lit) }
func (p *Parser) parseThree() (int, error) { _, lit := p.scan(); return strconv.Atoi(lit) }
func (p *Parser) parseFour() (int, error)  { _, lit := p.scan(); return strconv.Atoi(lit) }
func (p *Parser) parseAt() (int, error) {
	tok, lit := p.scan()
	nextTok, nextLit := p.scan()
	switch {
	case tok == 1 && nextTok == 2:
		return strconv.Atoi(lit)
	case tok == 3 && nextTok == 1:
		return strconv.Atoi(nextLit)
	}
	return 0, nil
}
