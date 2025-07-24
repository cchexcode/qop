ALTER TABLE "ASD" ADD COLUMN "ASD" TEXT FOREIGN KEY REFERENCES "X"("Y");
DROP TABLE "boosker";-- This is a header comment
ALTER TABLE "users" ADD COLUMN "email" TEXT; -- inline comment

/* 
Multi-line comment
*/
DROP TABLE "old_table";

CREATE TABLE "new_table" (
    id SERIAL PRIMARY KEY, -- column comment
    name VARCHAR(255) /* another inline comment */
);