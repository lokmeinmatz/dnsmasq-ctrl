




clean:
	rm -rf frontend/dist
	rm -rf target

full-clean: clean
	rm -rf frontend/node_modules

package: full-clean
ifdef name
	mkdir bundle || true
	tar cfv bundle/$(name) --exclude=bundle .
else
	echo "declare name=asdf.tar"
endif