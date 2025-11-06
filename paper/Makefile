.PHONY: all figures clean

FIGURES_DIR := $(abspath figures)

all: paper.pdf

figures:
	python3 ../scripts/make_figures.py --figures "$(FIGURES_DIR)"

paper.pdf: main.md refs.bib
	bash ../scripts/build_paper.sh

clean:
	rm -f paper.pdf
	rm -f figures/*.pdf
