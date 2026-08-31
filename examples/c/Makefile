CC ?= cc
INC = ../../crates/rsact-ffi/include
SRC = ./
CFLAGS += -I$(INC)
LDLIBS += ../../target/release/librsact_ffi.a

BINS = basic animate input

all: $(BINS)

$(BINS): %: %.c
	$(CC) $(CFLAGS) $< $(LDLIBS) -o build/$@

clean:
	rm -f $(BINS)

cp:
	cp $(INC)/* $(SRC)

.PHONY: all clean
