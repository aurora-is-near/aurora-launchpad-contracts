generate-pdf:
	@pandoc article.md -o article.pdf -s --citeproc --pdf-engine=xelatex

generate-tex:
	@rm -rf main.*
	@pandoc article.md -o main.tex -s --citeproc
	@xelatex main.tex
	@bibtex main
	@xelatex main.tex
	@xelatex main.tex

pdflatext:
	@rm -rf main.*
	@pandoc article.md --bibliography=references.bib -s --natbib -o main.tex
	@echo "\\typeout{get arXiv to do 4 passes: Label(s) may have changed. Rerun}" >> main.tex
	@pdflatex main.tex
	@bibtex main
	@pdflatex main.tex
	@pdflatex main.tex
	@tar -czf ax.tar.gz main.tex main.bbl
tar:
	@rm ax.tar.gz
	@tar -czf ax.tar.gz main.tex main.bbl
