typedef enum rs_Suit {
  spade = 0,
  heart = 1,
  club = 2,
  diamond = 3,
  joker = 4,
} rs_Suit;

typedef enum rs_TexasType {
  no_calc,
  high_card,
  one_pair,
  two_pair,
  three,
  straight,
  flush,
  full_house,
  four,
  straight_flush,
  royal_flush,
} rs_TexasType;

typedef struct rs_GinRummyCards rs_GinRummyCards;

typedef struct rs_PokerCards rs_PokerCards;

typedef struct rs_TexasCards rs_TexasCards;

typedef struct rs_PokerCard {
  enum rs_Suit suit;
  uint8_t number;
} rs_PokerCard;

typedef struct rs_CardBuffer {
  struct rs_PokerCard *data;
  uintptr_t len;
} rs_CardBuffer;

typedef struct rs_Counter {
  enum rs_Suit t;
  uint8_t n;
  uint8_t bucket[14];
} rs_Counter;

typedef struct rs_TexasCardBuffer {
  struct rs_CardBuffer cardbuf;
  enum rs_TexasType texas;
  uint64_t score;
} rs_TexasCardBuffer;

struct rs_GinRummyCards *rs_GinRummyCards_new(void);

void rs_GinRummyCards_free(struct rs_GinRummyCards *p_pcs);

int8_t rs_GinRummyCards_sort(struct rs_GinRummyCards *p_pcs, uint8_t *p_out);

int8_t rs_GinRummyCards_assign(struct rs_GinRummyCards *p_pcs,
                               const uint16_t *p_data,
                               uintptr_t data_len,
                               uint8_t freeze,
                               uint8_t *p_out);

struct rs_PokerCards *rs_PokerCards_new(void);

void rs_PokerCards_free(struct rs_PokerCards *p_pcs);

int8_t rs_PokerCards_assign(struct rs_PokerCards *p_pcs,
                            const uint16_t *p_data,
                            uintptr_t data_len);

struct rs_CardBuffer rs_PokerCards_get_cards(struct rs_PokerCards *p_pcs);

void rs_CardBuffer_free(struct rs_CardBuffer buf);

struct rs_Counter *rs_PokerCards_get_counter(struct rs_PokerCards *p_stu, enum rs_Suit s);

struct rs_Counter *rs_Counter_new(enum rs_Suit s);

void rs_Counter_free(struct rs_Counter *p_counter);

struct rs_PokerCard *rs_PokerCard_new(uint16_t n);

void rs_PokerCard_free(struct rs_PokerCard *p_poker);

struct rs_TexasCards *rs_TexasCards_new(void);

void rs_TexasCards_free(struct rs_TexasCards *p_poker);

int8_t rs_TexasCards_assign(struct rs_TexasCards *p_pcs,
                            const uint16_t *p_data,
                            uintptr_t data_len);

struct rs_TexasCardBuffer rs_TexasCards_get_best(struct rs_TexasCards *p_pcs);

void rs_TexasCardBuffer_free(struct rs_TexasCardBuffer buf);
