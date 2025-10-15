generate-pdf:
	@pandoc article.md -o article.pdf -s --citeproc --pdf-engine=xelatex

generate-tex:
	@rm -rf main.*
	@pandoc article.md -o main.tex -s --citeproc
	@xelatex main.tex
	@bibtex main
	@xelatex main.tex
	@xelatex main.tex
